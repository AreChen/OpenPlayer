//! Single-producer/single-consumer transport. Kernel auto-reset events transfer
//! exclusive ownership of one pixel slot; aligned atomics carry epoch/close state.
//! Peers are trusted native processes and must obey this protocol. This is not a
//! sandbox against a process intentionally modifying another peer's mapped bytes.
use super::*;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering, fence};
use std::{cell::Cell, marker::PhantomData, ptr, slice, sync::Arc};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
        WAIT_OBJECT_0, WAIT_TIMEOUT,
    },
    System::{
        Memory::{
            CreateFileMappingW, FILE_MAP_READ, FILE_MAP_WRITE, MEMORY_MAPPED_VIEW_ADDRESS,
            MapViewOfFile, OpenFileMappingW, PAGE_READWRITE, UnmapViewOfFile,
        },
        Performance::{QueryPerformanceCounter, QueryPerformanceFrequency},
        Threading::{
            CreateEventW, EVENT_MODIFY_STATE, OpenEventW, SYNCHRONIZATION_SYNCHRONIZE, SetEvent,
            WaitForSingleObject,
        },
    },
};

const MAGIC: &[u8; 8] = b"OPPRES01";
const VERSION: u32 = 1;
const EPOCH_OFFSET: usize = 16;
const CLOSED_OFFSET: usize = 24;
const CLAIM_OFFSET: usize = 28;

struct Handle(HANDLE);
// Windows kernel handles may be used and closed on different threads. Arc keeps
// event handles alive while a control token or frame lease can still use them.
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
impl Handle {
    fn checked(raw: HANDLE) -> io::Result<Self> {
        if raw.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(raw))
        }
    }
    fn fresh(raw: HANDLE) -> io::Result<Self> {
        let existed = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        let handle = Self::checked(raw)?;
        if existed {
            Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "presentation object exists",
            ))
        } else {
            Ok(handle)
        }
    }
    fn signal(&self) -> io::Result<()> {
        if unsafe { SetEvent(self.0) } == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    fn wait(&self, timeout_ms: u32) -> io::Result<bool> {
        match unsafe { WaitForSingleObject(self.0, timeout_ms) } {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => Err(io::Error::last_os_error()),
        }
    }
}

struct Mapping {
    view: MEMORY_MAPPED_VIEW_ADDRESS,
    _handle: Handle,
    capacity: u32,
}
// The fixed prefix is immutable after creation. Control words are atomic;
// remaining bytes are accessed only while owning the free/ready event token.
unsafe impl Send for Mapping {}
unsafe impl Sync for Mapping {}
impl Drop for Mapping {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(self.view);
        }
    }
}
impl Mapping {
    fn map(handle: Handle, capacity: u32) -> io::Result<Self> {
        let view = unsafe {
            MapViewOfFile(
                handle.0,
                FILE_MAP_READ | FILE_MAP_WRITE,
                0,
                0,
                capacity as usize + PAYLOAD_OFFSET,
            )
        };
        if view.Value.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            view,
            _handle: handle,
            capacity,
        })
    }
    fn base(&self) -> *mut u8 {
        self.view.Value.cast()
    }
    fn epoch(&self) -> &AtomicU64 {
        // MapViewOfFile is allocation-granularity aligned; offset is 8-aligned.
        unsafe { &*self.base().add(EPOCH_OFFSET).cast::<AtomicU64>() }
    }
    fn closed(&self) -> &AtomicU32 {
        unsafe { &*self.base().add(CLOSED_OFFSET).cast::<AtomicU32>() }
    }
    fn claimed(&self) -> &AtomicU32 {
        unsafe { &*self.base().add(CLAIM_OFFSET).cast::<AtomicU32>() }
    }
}

/// Cloneable cancellation token. Epoch changes invalidate pending work without
/// waiting for a frame lease or GPU. Check `is_current` again before presentation.
#[derive(Clone)]
pub struct Control {
    mapping: Arc<Mapping>,
    ready: Arc<Handle>,
}
impl Control {
    pub fn epoch(&self) -> u64 {
        self.mapping.epoch().load(Ordering::Acquire)
    }
    pub fn is_closed(&self) -> bool {
        self.mapping.closed().load(Ordering::Acquire) != 0
    }
    pub fn is_current(&self, epoch: u64) -> bool {
        !self.is_closed() && epoch == self.epoch()
    }
    pub fn advance_epoch(&self) -> u64 {
        match self
            .mapping
            .epoch()
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |epoch| {
                epoch.checked_add(1)
            }) {
            Ok(previous) => previous + 1,
            Err(_) => {
                self.close();
                u64::MAX
            }
        }
    }
    pub fn close(&self) {
        self.mapping.closed().store(1, Ordering::Release);
        let _ = self.ready.signal();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendOutcome {
    Sent,
    Busy,
    Stale,
    Closed,
}

/// Nonblocking producer. Busy means discard this frame, not queue it elsewhere.
pub struct Producer {
    endpoint: Endpoint,
    control: Control,
    free: Handle,
    last_sequence: u64,
    _not_sync: PhantomData<Cell<()>>,
}
impl Producer {
    pub fn create(capacity_bytes: u32) -> io::Result<Self> {
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|error| io::Error::other(error.to_string()))?;
        let token: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        let endpoint = Endpoint {
            protocol: PROTOCOL.into(),
            mapping: format!("{NAME_PREFIX}{token}"),
            capacity_bytes,
            qpc_frequency: qpc_frequency()?,
            producer_pid: std::process::id(),
        };
        endpoint.validate()?;
        let name = wide(&endpoint.mapping);
        let handle = Handle::fresh(unsafe {
            CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                ptr::null(),
                PAGE_READWRITE,
                0,
                capacity_bytes + PAYLOAD_OFFSET as u32,
                name.as_ptr(),
            )
        })?;
        let mapping = Mapping::map(handle, capacity_bytes)?;
        // Newly created page-file mappings are zero-filled and have no peer yet.
        unsafe {
            ptr::copy_nonoverlapping(MAGIC.as_ptr(), mapping.base(), MAGIC.len());
            ptr::copy_nonoverlapping(VERSION.to_le_bytes().as_ptr(), mapping.base().add(8), 4);
            ptr::copy_nonoverlapping(
                capacity_bytes.to_le_bytes().as_ptr(),
                mapping.base().add(12),
                4,
            );
            ptr::write(
                mapping.base().add(EPOCH_OFFSET).cast::<AtomicU64>(),
                AtomicU64::new(1),
            );
            ptr::write(
                mapping.base().add(CLOSED_OFFSET).cast::<AtomicU32>(),
                AtomicU32::new(0),
            );
            ptr::write(
                mapping.base().add(CLAIM_OFFSET).cast::<AtomicU32>(),
                AtomicU32::new(0),
            );
        }
        let ready = Arc::new(create_event(&endpoint.mapping, "ready", false)?);
        let free = create_event(&endpoint.mapping, "free", true)?;
        Ok(Self {
            endpoint,
            control: Control {
                mapping: Arc::new(mapping),
                ready,
            },
            free,
            last_sequence: 0,
            _not_sync: PhantomData,
        })
    }
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
    pub fn control(&self) -> Control {
        self.control.clone()
    }
    pub fn try_send(&mut self, meta: FrameMeta, rgba: &[u8]) -> io::Result<SendOutcome> {
        let header = meta.encode(self.endpoint.capacity_bytes)?;
        if meta.payload_bytes()? != rgba.len() || meta.sequence <= self.last_sequence {
            return Err(invalid(
                "invalid presentation payload or non-increasing sequence",
            ));
        }
        if self.control.is_closed() {
            return Ok(SendOutcome::Closed);
        }
        if !self.control.is_current(meta.epoch) {
            return Ok(SendOutcome::Stale);
        }
        if !self.free.wait(0)? {
            return Ok(SendOutcome::Busy);
        }
        fence(Ordering::Acquire);
        // The free token grants exclusive write access until ready is signalled.
        unsafe {
            let base = self.control.mapping.base();
            ptr::copy_nonoverlapping(header.as_ptr(), base.add(GLOBAL_SIZE), FRAME_SIZE);
            ptr::copy_nonoverlapping(rgba.as_ptr(), base.add(PAYLOAD_OFFSET), rgba.len());
        }
        fence(Ordering::Release);
        if let Err(error) = self.control.ready.signal() {
            self.control.close();
            return Err(error);
        }
        self.last_sequence = meta.sequence;
        Ok(SendOutcome::Sent)
    }
}
impl Drop for Producer {
    fn drop(&mut self) {
        self.control.close();
    }
}

pub struct Consumer {
    control: Control,
    free: Handle,
    last_sequence: u64,
}
impl Consumer {
    /// Open once per endpoint. Reattachment must use a fresh producer/endpoint.
    pub fn open(endpoint: &Endpoint) -> io::Result<Self> {
        endpoint.validate()?;
        if endpoint.qpc_frequency != qpc_frequency()? {
            return Err(invalid("presentation QPC frequency mismatch"));
        }
        let name = wide(&endpoint.mapping);
        let handle = Handle::checked(unsafe {
            OpenFileMappingW(FILE_MAP_READ | FILE_MAP_WRITE, 0, name.as_ptr())
        })?;
        let mapping = Mapping::map(handle, endpoint.capacity_bytes)?;
        let mut prefix = [0u8; 16];
        unsafe {
            ptr::copy_nonoverlapping(mapping.base(), prefix.as_mut_ptr(), prefix.len());
        }
        if &prefix[0..8] != MAGIC
            || prefix[8..12] != VERSION.to_le_bytes()
            || prefix[12..16] != endpoint.capacity_bytes.to_le_bytes()
        {
            return Err(invalid("presentation mapping header mismatch"));
        }
        let ready = Arc::new(open_event(&endpoint.mapping, "ready")?);
        let free = open_event(&endpoint.mapping, "free")?;
        if mapping
            .claimed()
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(invalid("presentation endpoint already has a consumer"));
        }
        let consumer = Self {
            control: Control {
                mapping: Arc::new(mapping),
                ready,
            },
            free,
            last_sequence: 0,
        };
        consumer.ensure_open()?;
        Ok(consumer)
    }
    pub fn control(&self) -> Control {
        self.control.clone()
    }
    fn ensure_open(&self) -> io::Result<()> {
        if self.control.is_closed() {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "presentation transport closed",
            ))
        } else {
            Ok(())
        }
    }
    /// At most one lease. None means timeout or an invalidated frame; close is
    /// BrokenPipe. Release the lease after copying/uploading, never retain it in a queue.
    pub fn receive(&mut self, timeout_ms: u32) -> io::Result<Option<FrameLease<'_>>> {
        if timeout_ms > 1000 {
            return Err(invalid("presentation wait exceeds 1000 ms"));
        }
        self.ensure_open()?;
        if !self.control.ready.wait(timeout_ms)? {
            return Ok(None);
        }
        self.ensure_open()?;
        fence(Ordering::Acquire);
        let mut header = [0; FRAME_SIZE];
        unsafe {
            ptr::copy_nonoverlapping(
                self.control.mapping.base().add(GLOBAL_SIZE),
                header.as_mut_ptr(),
                FRAME_SIZE,
            );
        }
        let meta = match FrameMeta::decode(&header, self.control.mapping.capacity) {
            Ok(meta) if meta.sequence > self.last_sequence => meta,
            _ => {
                self.control.close();
                return Err(invalid("invalid presentation frame received"));
            }
        };
        self.last_sequence = meta.sequence;
        if !self.control.is_current(meta.epoch) {
            fence(Ordering::Release);
            self.free.signal()?;
            return Ok(None);
        }
        Ok(Some(FrameLease {
            consumer: self,
            meta,
        }))
    }
}
impl Drop for Consumer {
    fn drop(&mut self) {
        self.control.close();
    }
}

/// The borrowed pixels cannot be overwritten until this lease is dropped.
pub struct FrameLease<'a> {
    consumer: &'a mut Consumer,
    meta: FrameMeta,
}
impl FrameLease<'_> {
    pub fn meta(&self) -> FrameMeta {
        self.meta
    }
    pub fn pixels(&self) -> &[u8] {
        // Metadata was checked before constructing the lease. Pixel slot belongs
        // exclusively to the consumer until Drop signals the free event.
        unsafe {
            slice::from_raw_parts(
                self.consumer.control.mapping.base().add(PAYLOAD_OFFSET),
                self.meta.stride as usize * self.meta.height as usize,
            )
        }
    }
    pub fn is_current(&self) -> bool {
        self.consumer.control.is_current(self.meta.epoch)
    }
}
impl Drop for FrameLease<'_> {
    fn drop(&mut self) {
        fence(Ordering::Release);
        if self.consumer.free.signal().is_err() {
            self.consumer.control.close();
        }
    }
}

pub fn qpc_now() -> io::Result<i64> {
    let mut value = 0;
    if unsafe { QueryPerformanceCounter(&mut value) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(value)
}
pub fn qpc_frequency() -> io::Result<i64> {
    let mut value = 0;
    if unsafe { QueryPerformanceFrequency(&mut value) } == 0 || value <= 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(value)
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn create_event(name: &str, suffix: &str, initial: bool) -> io::Result<Handle> {
    let name = wide(&format!("{name}-{suffix}"));
    Handle::fresh(unsafe { CreateEventW(ptr::null(), 0, i32::from(initial), name.as_ptr()) })
}
fn open_event(name: &str, suffix: &str) -> io::Result<Handle> {
    let name = wide(&format!("{name}-{suffix}"));
    Handle::checked(unsafe {
        OpenEventW(
            EVENT_MODIFY_STATE | SYNCHRONIZATION_SYNCHRONIZE,
            0,
            name.as_ptr(),
        )
    })
}

#[cfg(test)]
mod tests;
