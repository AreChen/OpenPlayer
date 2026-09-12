# Native presentation transport

Enable the Rust crate's `presentation` feature. Windows exposes `Producer`,
`Consumer`, `Control`, and `FrameLease` under `presentation::windows`. This is an
experimental native transport, not a published host adapter or a JS API.

## Frame ownership

1. The host creates `Producer::create(capacity_bytes)` and privately sends its
   serialized `endpoint()` to the native process over the existing control RPC.
2. The process calls `Consumer::open` once. A second consumer is rejected. Reopen
   requires a new endpoint, including after a process crash.
3. The host calls `try_send(meta, rgba)`. `Busy` means discard the input, not add
   it to another queue. There is exactly one outstanding frame, bounded to the
   negotiated capacity (maximum 3840 x 2160 x 4 bytes).
4. `receive(timeout_ms)` returns a lease, no frame, or an error. The maximum wait
   is 1000 ms. Drop the lease after uploading/copying its pixels to release the
   slot. Never keep leases in a queue. Check `is_current()` before presentation.

Pixels are tightly packed 8-bit RGBA, top-down, width/height at least 16 and at
most 3840/2160. This byte layout does not perform HDR conversion. The producer
must supply SDR BT.709/sRGB-compatible content with opaque alpha. This initial
transport copies CPU pixels; it is not GPU zero-copy.

`FrameMeta` has a globally increasing nonzero sequence, nonzero epoch, target
QPC ticks, duration in nanoseconds (0 < duration <= 10 seconds), dimensions,
stride, and flags. Sequence gaps require the presenter to reset temporal history.
`RESET` explicitly resets history; `REDRAW` and `REPEAT` must not be counted as
new source frames or used to generate a new interpolation interval.

## Clock and cancellation

QPC is shared across local processes; `qpc_now()` and the endpoint's frequency
are the clock contract. Do not send mpv's process-relative timestamps unchanged.
Convert them using a same-process mpv/QPC clock sample in the producer. Frame
duration is distinct from both the target timestamp and the display refresh rate.

The host advances `Control::advance_epoch()` on pause, seek, output replacement,
or other history-invalidating changes. Pending stale frames are discarded; an
already leased frame becomes non-current. This does not cancel a GPU submission
already in progress or retract an image already presented.

Dropping either peer closes the transport. Explicit `Control::close()` is
idempotent, invalidates leases and wakes a waiting consumer with `BrokenPipe`.
Control clones retain the mapped objects until dropped. A hard-crashed process
cannot execute Drop: the native-process supervisor must close the session and
terminate/join its workers. The producer remains nonblocking even if a crashed
consumer held the slot. Do not synthesize a free token to recover that endpoint.

## Wire and trust

The endpoint uses protocol `openplayer-present-rgba-v1`, a random 128-bit token
in `Local\OpenPlayer-Present-<token>`, capacity, QPC frequency and producer PID.
The auto-reset event names append `-ready` and `-free`. Objects use the process's
default Windows security descriptor, are not inherited, and reject name
collisions. Keep endpoint values private to the session.

The mapping contains a 64-byte global header, a 64-byte frame header, then pixels.
Integers are little-endian. Global offsets: magic `OPPRES01` at 0, version u32 at
8, capacity u32 at 12, epoch atomic u64 at 16, closed atomic u32 at 24, and the
one-time consumer claim atomic u32 at 28. Frame offsets: sequence u64 at 0, epoch
u64 at 8, target QPC i64 at 16, duration u64 at 24, width/height/stride/payload
length u32 at 32/36/40/44, flags u32 at 48. Reserved bytes are zero. Events transfer
exclusive ownership of frame bytes; only the control words are concurrently
accessed atomically. The implementation validates sizes before constructing a
pixel slice and closes on malformed received headers.

Both sides are trusted native code. This is not an isolation boundary against a
malicious process rewriting shared memory or impersonating the producer PID.
GPU selection, parent window authorization, presentation scheduling, DLL trust,
and vendor SDK lifetime remain the responsibility of the host/presenter adapter.
