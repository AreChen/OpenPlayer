use std::sync::atomic::{AtomicBool, Ordering};
use tokio::process::{Child, Command};

pub(super) struct ProcessTree {
    stopped: AtomicBool,
    #[cfg(windows)]
    job: isize,
    #[cfg(unix)]
    pid: i32,
}

pub(super) fn configure(command: &mut Command) {
    #[cfg(windows)]
    command.creation_flags(0x08000004); // CREATE_NO_WINDOW | CREATE_SUSPENDED
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.as_std_mut().process_group(0);
    }
}

impl ProcessTree {
    pub(super) fn attach(child: &Child) -> Result<Self, String> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};
            let handle = child
                .raw_handle()
                .ok_or("native process handle unavailable")?;
            let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if job.is_null() {
                return Err("cannot create native process job".into());
            }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if unsafe {
                SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    &limits as *const _ as _,
                    std::mem::size_of_val(&limits) as u32,
                )
            } == 0
                || unsafe { AssignProcessToJobObject(job, handle as _) } == 0
            {
                unsafe {
                    CloseHandle(job);
                }
                return Err("cannot contain native process in a job".into());
            }
            let tree = Self {
                stopped: AtomicBool::new(false),
                job: job as isize,
            };
            resume_initial_thread(child.id().ok_or("native process exited")?)?;
            Ok(tree)
        }
        #[cfg(unix)]
        {
            Ok(Self {
                stopped: AtomicBool::new(false),
                pid: child.id().ok_or("native process exited")? as i32,
            })
        }
    }

    pub(super) fn stop(&self) {
        if self.stopped.swap(true, Ordering::SeqCst) {
            return;
        }
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.job as _, 1);
        }
        #[cfg(unix)]
        unsafe {
            libc::kill(-self.pid, libc::SIGKILL);
        }
    }

    pub(super) fn stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    #[cfg(all(test, windows))]
    pub(super) fn job_handle(&self) -> windows_sys::Win32::Foundation::HANDLE {
        self.job as _
    }

    pub(super) fn wait_stopped(&self) -> Result<(), String> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::JobObjects::*;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            loop {
                let mut info: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION =
                    unsafe { std::mem::zeroed() };
                if unsafe {
                    QueryInformationJobObject(
                        self.job as _,
                        JobObjectBasicAccountingInformation,
                        &mut info as *mut _ as _,
                        std::mem::size_of_val(&info) as u32,
                        std::ptr::null_mut(),
                    )
                } == 0
                {
                    return Err("cannot verify native process cleanup".into());
                }
                if info.ActiveProcesses == 0 {
                    break;
                }
                if std::time::Instant::now() >= deadline {
                    return Err("native processes have not exited; retry plugin operation".into());
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
fn resume_initial_thread(pid: u32) -> Result<(), String> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::{
            Diagnostics::ToolHelp::*,
            Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME},
        },
    };
    // Stable Rust does not expose CreateProcess's primary thread handle. A newly
    // CREATE_SUSPENDED process has one initial thread; reject ambiguous snapshots.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err("cannot inspect suspended native process".into());
    }
    let mut entry: THREADENTRY32 = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of_val(&entry) as u32;
    let mut threads = Vec::new();
    let mut found = unsafe { Thread32First(snapshot, &mut entry) };
    while found != 0 {
        if entry.th32OwnerProcessID == pid {
            threads.push(entry.th32ThreadID);
        }
        found = unsafe { Thread32Next(snapshot, &mut entry) };
    }
    unsafe {
        CloseHandle(snapshot);
    }
    if threads.len() != 1 {
        return Err("cannot identify suspended native primary thread".into());
    }
    let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, threads[0]) };
    if thread.is_null() {
        return Err("cannot open suspended native thread".into());
    }
    let previous = unsafe { ResumeThread(thread) };
    unsafe {
        CloseHandle(thread);
    }
    if previous != 1 {
        return Err("cannot resume native process after job assignment".into());
    }
    Ok(())
}

impl Drop for ProcessTree {
    fn drop(&mut self) {
        self.stop();
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.job as _);
        }
    }
}
