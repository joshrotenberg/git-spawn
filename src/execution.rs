//! Subprocess lifetime and bounded byte capture.
//!
//! A caller must put Unix children in their own process group and enable
//! `kill_on_drop`. Windows children are spawned suspended here, assigned to a
//! Job Object, then resumed. Cancellation and timeout terminate that owned
//! group/job and await the direct child. Dropping the execution future still
//! requests termination, but async reaping cannot be guaranteed from `Drop`.
//! Children which deliberately leave the process group or Job Object, and
//! children of a parent killed with SIGKILL, are outside these guarantees.

use crate::error::{Error, Result};
use crate::output::{CommandOutput, ProcessStatus};
use std::fmt;
use std::future::pending;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::Notify;
use tokio::time::{Instant, sleep_until};

/// A cloneable, one-way cancellation latch.
#[derive(Clone, Default, Debug)]
pub struct CancellationToken(Arc<CancellationInner>);

#[derive(Default, Debug)]
struct CancellationInner {
    cancelled: AtomicBool,
    notify: Notify,
}

impl CancellationToken {
    /// Create a token in its active state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wake current and future waiters. Repeated calls have no effect.
    pub fn cancel(&self) {
        if !self.0.cancelled.swap(true, Ordering::SeqCst) {
            self.0.notify.notify_waiters();
        }
    }

    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::SeqCst)
    }

    /// Wait until this token is cancelled.
    pub async fn cancelled(&self) {
        loop {
            let notified = self.0.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

/// Maximum retained bytes per output stream. `None` means unlimited. Buffer
/// allocations may slightly exceed a limit because the allocator rounds
/// capacity, while the retained byte length never exceeds it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OutputLimits {
    /// Maximum retained stdout bytes.
    pub stdout: Option<usize>,
    /// Maximum retained stderr bytes.
    pub stderr: Option<usize>,
}

/// Identifies a captured output pipe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputStream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// Why an execution stopped before normal pipe EOF and child completion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionFailureKind {
    /// A cancellation token was triggered.
    Cancelled,
    /// The complete execution exceeded its configured deadline.
    TimedOut {
        /// Configured execution timeout.
        timeout: Duration,
    },
    /// Writing or closing stdin failed.
    Input {
        /// OS error without input contents.
        message: String,
    },
    /// Reading one output pipe failed.
    Read {
        /// Pipe that failed.
        stream: OutputStream,
        /// OS error without output contents.
        message: String,
    },
    /// One output pipe exceeded its byte limit.
    CaptureLimit {
        /// Pipe whose limit was exceeded.
        stream: OutputStream,
        /// Configured byte limit.
        limit: usize,
    },
    /// Awaiting the direct child failed.
    Wait {
        /// OS error without output contents.
        message: String,
    },
    /// Job Object setup, resume, or release failed.
    Containment {
        /// OS error without output contents.
        message: String,
    },
}

/// Result of trying to terminate the descendants owned by this invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminationOutcome {
    /// No termination was necessary.
    NotRequested,
    /// The OS accepted a group or job termination request.
    Signaled,
    /// The owned process group was already absent or its leader was reaped.
    AlreadyExited,
    /// No group or job was available to address.
    Unavailable,
    /// The OS rejected a termination request.
    Failed {
        /// OS error without captured output.
        message: String,
    },
}

/// Evidence from handled cleanup. `Signaled` records a successful OS request,
/// not proof that every descendant has exited.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupReport {
    /// Whether a wait collected the direct child's exit status.
    pub direct_child_reaped: bool,
    /// Whether a direct-child kill request was accepted.
    pub direct_child_kill_requested: bool,
    /// Error from awaiting the child during cleanup, if any.
    pub direct_child_wait_error: Option<String>,
    /// Outcome of process group or Job Object termination.
    pub process_tree: TerminationOutcome,
    /// Whether the cleanup deadline elapsed before the child was reaped.
    pub wait_timed_out: bool,
}

impl Default for CleanupReport {
    fn default() -> Self {
        Self {
            direct_child_reaped: false,
            direct_child_kill_requested: false,
            direct_child_wait_error: None,
            process_tree: TerminationOutcome::NotRequested,
            wait_timed_out: false,
        }
    }
}

/// Pipe EOF and truncation observations. EOF can coexist with truncation when
/// cleanup drains a pipe after its configured byte limit was reached.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CaptureReport {
    /// Whether stdout reached pipe EOF.
    pub stdout_pipe_eof: bool,
    /// Whether stderr reached pipe EOF.
    pub stderr_pipe_eof: bool,
    /// Whether stdout bytes were discarded after reaching its limit.
    pub stdout_truncated: bool,
    /// Whether stderr bytes were discarded after reaching its limit.
    pub stderr_truncated: bool,
}

/// Structured interrupted execution, preserving raw bytes for inspection.
#[derive(Clone)]
pub struct ExecutionFailure {
    /// Reason execution stopped.
    pub kind: ExecutionFailureKind,
    /// Direct-child status if it was reaped.
    pub status: Option<ProcessStatus>,
    /// Captured stdout bytes, possibly partial.
    pub stdout: Vec<u8>,
    /// Captured stderr bytes, possibly partial.
    pub stderr: Vec<u8>,
    /// Observed cleanup actions and outcomes.
    pub cleanup: CleanupReport,
    /// Pipe completion observations.
    pub capture: CaptureReport,
}

impl fmt::Debug for ExecutionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecutionFailure")
            .field("kind", &self.kind_label())
            .field("status", &self.status)
            .field("stdout_len", &self.stdout.len())
            .field("stderr_len", &self.stderr.len())
            .field("direct_child_reaped", &self.cleanup.direct_child_reaped)
            .field("process_tree", &self.process_tree_label())
            .field("wait_timed_out", &self.cleanup.wait_timed_out)
            .field("capture", &self.capture)
            .finish()
    }
}

impl fmt::Display for ExecutionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "git execution failed: {}", self.kind_label())
    }
}

impl ExecutionFailure {
    fn kind_label(&self) -> &'static str {
        match self.kind {
            ExecutionFailureKind::Cancelled => "cancelled",
            ExecutionFailureKind::TimedOut { .. } => "timed out",
            ExecutionFailureKind::Input { .. } => "stdin failure",
            ExecutionFailureKind::Read { .. } => "output read failure",
            ExecutionFailureKind::CaptureLimit { .. } => "output limit exceeded",
            ExecutionFailureKind::Wait { .. } => "child wait failure",
            ExecutionFailureKind::Containment { .. } => "process containment failure",
        }
    }

    fn process_tree_label(&self) -> &'static str {
        match self.cleanup.process_tree {
            TerminationOutcome::NotRequested => "not requested",
            TerminationOutcome::Signaled => "signaled",
            TerminationOutcome::AlreadyExited => "already exited",
            TerminationOutcome::Unavailable => "unavailable",
            TerminationOutcome::Failed { .. } => "failed",
        }
    }
}

struct OwnedChild {
    child: Child,
    #[cfg(unix)]
    pid: Option<u32>,
    reaped: bool,
    armed: bool,
    #[cfg(windows)]
    job: Option<WindowsJob>,
}

impl OwnedChild {
    fn new(child: Child) -> Self {
        #[cfg(unix)]
        let pid = child.id();
        Self {
            child,
            #[cfg(unix)]
            pid,
            reaped: false,
            armed: true,
            #[cfg(windows)]
            job: None,
        }
    }

    fn terminate_tree(&self) -> TerminationOutcome {
        #[cfg(windows)]
        {
            return match &self.job {
                Some(job) => job.terminate(),
                None => TerminationOutcome::Unavailable,
            };
        }
        #[cfg(unix)]
        {
            // A reaped leader no longer pins its pid. Avoid signalling a
            // process group number that could have been reused.
            if self.reaped {
                return TerminationOutcome::AlreadyExited;
            }
            match self.pid {
                Some(pid) => kill_process_group(pid),
                None => TerminationOutcome::Unavailable,
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            TerminationOutcome::Unavailable
        }
    }

    fn disarm(&mut self) -> io::Result<()> {
        #[cfg(windows)]
        if let Some(job) = &self.job {
            job.disarm()?;
        }
        self.armed = false;
        Ok(())
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.armed {
            self.terminate_tree();
            if !self.reaped {
                let _ = self.child.start_kill();
            }
            // The armed Windows Job Object kills members on close as a
            // fallback. Tokio's kill_on_drop also covers the direct child.
        }
    }
}

/// Run a configured command, concurrently writing stdin and draining both
/// pipes. The timeout spans stdin, pipe EOF and the direct-child wait.
pub(crate) async fn run(
    mut command: Command,
    input: Option<&[u8]>,
    timeout: Option<Duration>,
    cancellation: Option<&CancellationToken>,
    limits: OutputLimits,
    cleanup_timeout: Duration,
) -> Result<CommandOutput> {
    let started = Instant::now();
    let deadline = timeout
        .map(|duration| {
            started.checked_add(duration).ok_or_else(|| {
                Error::invalid_config("command timeout is too large for an instant deadline")
            })
        })
        .transpose()?;
    if started.checked_add(cleanup_timeout).is_none() {
        return Err(Error::invalid_config(
            "cleanup timeout is too large for an instant deadline",
        ));
    }
    if cancellation.is_some_and(CancellationToken::is_cancelled) {
        return Err(Error::Execution {
            failure: Box::new(ExecutionFailure {
                kind: ExecutionFailureKind::Cancelled,
                status: None,
                stdout: Vec::new(),
                stderr: Vec::new(),
                cleanup: CleanupReport::default(),
                capture: CaptureReport::default(),
            }),
        });
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::System::Threading::CREATE_SUSPENDED;
        command.as_std_mut().creation_flags(CREATE_SUSPENDED);
    }
    let child = command.spawn().map_err(map_spawn_error)?;
    let mut owned = OwnedChild::new(child);
    #[cfg(windows)]
    {
        match WindowsJob::assign(&owned.child) {
            Ok(job) => owned.job = Some(job),
            Err(error) => {
                let kind = ExecutionFailureKind::Containment {
                    message: error.to_string(),
                };
                return Err(cleanup_without_pipes(&mut owned, kind, cleanup_timeout).await);
            }
        }
        if let Err(error) = resume_process(&owned.child) {
            let kind = ExecutionFailureKind::Containment {
                message: error.to_string(),
            };
            return Err(cleanup_without_pipes(&mut owned, kind, cleanup_timeout).await);
        }
    }

    let mut stdout = owned.child.stdout.take().expect("stdout must be piped");
    let mut stderr = owned.child.stderr.take().expect("stderr must be piped");
    let mut stdin = owned.child.stdin.take();
    let input = input.unwrap_or_default();
    let mut input_pos = 0;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut capture = CaptureReport::default();
    let mut status = None;
    let failure_kind = loop {
        if status.is_some() && capture.stdout_pipe_eof && capture.stderr_pipe_eof && stdin.is_none()
        {
            break None;
        }
        if cancellation.is_some_and(CancellationToken::is_cancelled) {
            break Some(ExecutionFailureKind::Cancelled);
        }
        if deadline.is_some_and(|end| Instant::now() >= end) {
            break Some(ExecutionFailureKind::TimedOut {
                timeout: timeout.expect("deadline has timeout"),
            });
        }
        let mut out_buf = [0u8; 8192];
        let mut err_buf = [0u8; 8192];
        tokio::select! {
            _ = wait_for_cancellation(cancellation) => break Some(ExecutionFailureKind::Cancelled),
            _ = wait_for_deadline(deadline) => break Some(ExecutionFailureKind::TimedOut { timeout: timeout.expect("deadline has timeout") }),
            read = stdout.read(&mut out_buf), if !capture.stdout_pipe_eof => {
                match read {
                    Ok(0) => capture.stdout_pipe_eof = true,
                    Ok(n) => {
                        if let Some(kind) = append_limited(&mut out, &out_buf[..n], limits.stdout, OutputStream::Stdout) {
                            capture.stdout_truncated = true;
                            break Some(kind);
                        }
                    }
                    Err(e) => break Some(ExecutionFailureKind::Read { stream: OutputStream::Stdout, message: e.to_string() }),
                }
            }
            read = stderr.read(&mut err_buf), if !capture.stderr_pipe_eof => {
                match read {
                    Ok(0) => capture.stderr_pipe_eof = true,
                    Ok(n) => {
                        if let Some(kind) = append_limited(&mut err, &err_buf[..n], limits.stderr, OutputStream::Stderr) {
                            capture.stderr_truncated = true;
                            break Some(kind);
                        }
                    }
                    Err(e) => break Some(ExecutionFailureKind::Read { stream: OutputStream::Stderr, message: e.to_string() }),
                }
            }
            write = write_input(&mut stdin, input, input_pos), if stdin.is_some() => {
                match write {
                    Ok(0) => stdin = None,
                    Ok(n) => input_pos += n,
                    Err(e) => break Some(ExecutionFailureKind::Input { message: e.to_string() }),
                }
            }
            // Keep the direct child unreaped until all I/O is complete. Its
            // pid then cannot be reused before a later interruption signals
            // the owned process group.
            waited = owned.child.wait(), if status.is_none() && capture.stdout_pipe_eof && capture.stderr_pipe_eof && stdin.is_none() => {
                match waited {
                    Ok(value) => {
                        owned.reaped = true;
                        status = Some(value);
                    }
                    Err(e) => break Some(ExecutionFailureKind::Wait { message: e.to_string() }),
                }
            }
        }
    };

    if let Some(kind) = failure_kind {
        drop(stdin.take());
        let mut cleanup = CleanupReport {
            process_tree: owned.terminate_tree(),
            ..CleanupReport::default()
        };
        if !owned.reaped {
            cleanup.direct_child_kill_requested = owned.child.start_kill().is_ok();
        }
        let cleanup_deadline = cleanup_deadline(cleanup_timeout);
        let mut out_finished = capture.stdout_pipe_eof;
        let mut err_finished = capture.stderr_pipe_eof;
        let mut wait_finished = owned.reaped;
        while (!wait_finished || !out_finished || !err_finished)
            && Instant::now() < cleanup_deadline
        {
            let mut out_buf = [0u8; 8192];
            let mut err_buf = [0u8; 8192];
            tokio::select! {
                _ = sleep_until(cleanup_deadline) => {
                    cleanup.wait_timed_out = !owned.reaped;
                    break;
                }
                read = stdout.read(&mut out_buf), if !out_finished => {
                    match read {
                        Ok(0) => {
                            capture.stdout_pipe_eof = true;
                            out_finished = true;
                        }
                        Ok(n) => capture.stdout_truncated |= append_partial(&mut out, &out_buf[..n], limits.stdout),
                        Err(_) => out_finished = true,
                    }
                }
                read = stderr.read(&mut err_buf), if !err_finished => {
                    match read {
                        Ok(0) => {
                            capture.stderr_pipe_eof = true;
                            err_finished = true;
                        }
                        Ok(n) => capture.stderr_truncated |= append_partial(&mut err, &err_buf[..n], limits.stderr),
                        Err(_) => err_finished = true,
                    }
                }
                waited = owned.child.wait(), if !wait_finished => {
                    wait_finished = true;
                    match waited {
                        Ok(value) => {
                            owned.reaped = true;
                            status = Some(value);
                        }
                        Err(error) => cleanup.direct_child_wait_error = Some(error.to_string()),
                    }
                }
            }
        }
        cleanup.direct_child_reaped = owned.reaped;
        if !wait_finished {
            cleanup.wait_timed_out = true;
        }
        return Err(Error::Execution {
            failure: Box::new(ExecutionFailure {
                kind,
                status: status.map(ProcessStatus::from_exit_status),
                stdout: out,
                stderr: err,
                cleanup,
                capture,
            }),
        });
    }

    if let Err(error) = owned.disarm() {
        let kind = ExecutionFailureKind::Containment {
            message: error.to_string(),
        };
        return Err(Error::Execution {
            failure: Box::new(ExecutionFailure {
                kind,
                status: status.map(ProcessStatus::from_exit_status),
                stdout: out,
                stderr: err,
                cleanup: CleanupReport {
                    direct_child_reaped: true,
                    process_tree: owned.terminate_tree(),
                    ..CleanupReport::default()
                },
                capture,
            }),
        });
    }
    Ok(CommandOutput::from_process_output(std::process::Output {
        status: status.expect("completed child has status"),
        stdout: out,
        stderr: err,
    }))
}

fn append_limited(
    dst: &mut Vec<u8>,
    bytes: &[u8],
    limit: Option<usize>,
    stream: OutputStream,
) -> Option<ExecutionFailureKind> {
    let exceeded = limit.filter(|&max| bytes.len() > max.saturating_sub(dst.len()));
    let _ = append_partial(dst, bytes, limit);
    exceeded.map(|limit| ExecutionFailureKind::CaptureLimit { stream, limit })
}

fn append_partial(dst: &mut Vec<u8>, bytes: &[u8], limit: Option<usize>) -> bool {
    let count = limit.map_or(bytes.len(), |max| {
        max.saturating_sub(dst.len()).min(bytes.len())
    });
    if let Some(max) = limit {
        let needed = dst.len() + count;
        if needed > dst.capacity() {
            // Grow geometrically for throughput, but never request capacity
            // beyond the configured limit. The allocator may round upward.
            let target = dst.capacity().saturating_mul(2).max(needed).min(max);
            dst.reserve_exact(target - dst.len());
        }
    }
    dst.extend_from_slice(&bytes[..count]);
    count < bytes.len()
}

async fn write_input(
    stdin: &mut Option<tokio::process::ChildStdin>,
    bytes: &[u8],
    offset: usize,
) -> io::Result<usize> {
    match stdin {
        Some(pipe) if offset < bytes.len() => {
            let n = pipe.write(&bytes[offset..]).await?;
            if n == 0 {
                Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "stdin pipe accepted no bytes",
                ))
            } else {
                Ok(n)
            }
        }
        Some(pipe) => {
            pipe.shutdown().await?;
            Ok(0)
        }
        None => pending().await,
    }
}

async fn wait_for_cancellation(token: Option<&CancellationToken>) {
    match token {
        Some(token) => token.cancelled().await,
        None => pending().await,
    }
}

async fn wait_for_deadline(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => pending().await,
    }
}

fn cleanup_deadline(timeout: Duration) -> Instant {
    let now = Instant::now();
    // The duration was validated before spawn. If the representable Instant
    // range changes while the process runs, expire cleanup immediately.
    now.checked_add(timeout).unwrap_or(now)
}

fn map_spawn_error(error: io::Error) -> Error {
    // ENOENT can refer to the configured working directory just as readily
    // as the executable. The OS error alone cannot prove Git is absent.
    Error::Io {
        message: format!("failed to spawn git: {error}"),
        source: error,
    }
}

#[cfg(unix)]
fn kill_process_group(pid: u32) -> TerminationOutcome {
    let Ok(pgid) = i32::try_from(pid) else {
        return TerminationOutcome::Unavailable;
    };
    // SAFETY: negative pid addresses a process group; no Rust memory is used.
    if unsafe { libc::kill(-pgid, libc::SIGKILL) } == 0 {
        TerminationOutcome::Signaled
    } else {
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            TerminationOutcome::AlreadyExited
        } else {
            TerminationOutcome::Failed {
                message: error.to_string(),
            }
        }
    }
}

#[cfg(windows)]
async fn cleanup_without_pipes(
    owned: &mut OwnedChild,
    kind: ExecutionFailureKind,
    timeout: Duration,
) -> Error {
    let mut cleanup = CleanupReport {
        process_tree: owned.terminate_tree(),
        ..CleanupReport::default()
    };
    cleanup.direct_child_kill_requested = owned.child.start_kill().is_ok();
    let status = match tokio::time::timeout_at(cleanup_deadline(timeout), owned.child.wait()).await
    {
        Ok(Ok(value)) => {
            owned.reaped = true;
            Some(ProcessStatus::from_exit_status(value))
        }
        Ok(Err(error)) => {
            cleanup.direct_child_wait_error = Some(error.to_string());
            None
        }
        Err(_) => {
            cleanup.wait_timed_out = true;
            None
        }
    };
    cleanup.direct_child_reaped = owned.reaped;
    Error::Execution {
        failure: Box::new(ExecutionFailure {
            kind,
            status,
            stdout: Vec::new(),
            stderr: Vec::new(),
            cleanup,
            capture: CaptureReport::default(),
        }),
    }
}

#[cfg(windows)]
fn resume_process(child: &Child) -> io::Result<()> {
    use std::mem::{size_of, zeroed};
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

    let pid = child
        .id()
        .ok_or_else(|| io::Error::other("spawned process has no id"))?;
    // SAFETY: returned system handle is owned and closed on all paths.
    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if raw == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw) };
    let mut entry: THREADENTRY32 = unsafe { zeroed() };
    entry.dwSize = size_of::<THREADENTRY32>() as u32;
    let mut found = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) } != 0;
    while found {
        if entry.th32OwnerProcessID == pid {
            let raw_thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
            if raw_thread.is_null() {
                return Err(io::Error::last_os_error());
            }
            let thread = unsafe { OwnedHandle::from_raw_handle(raw_thread) };
            if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
                return Err(io::Error::last_os_error());
            }
            return Ok(());
        }
        found = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) } != 0;
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "suspended process thread not found",
    ))
}

#[cfg(windows)]
struct WindowsJob {
    handle: std::os::windows::io::OwnedHandle,
}

#[cfg(windows)]
impl WindowsJob {
    fn assign(child: &Child) -> io::Result<Self> {
        use std::os::windows::io::{AsRawHandle, FromRawHandle};
        use std::ptr::null;
        use windows_sys::Win32::System::JobObjects::{AssignProcessToJobObject, CreateJobObjectW};
        let raw = unsafe { CreateJobObjectW(null(), null()) };
        if raw.is_null() {
            return Err(io::Error::last_os_error());
        }
        let job = Self {
            handle: unsafe { std::os::windows::io::OwnedHandle::from_raw_handle(raw) },
        };
        job.set_kill_on_close(true)?;
        let process = child
            .raw_handle()
            .ok_or_else(|| io::Error::other("spawned process has no handle"))?;
        if unsafe { AssignProcessToJobObject(job.handle.as_raw_handle(), process) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(job)
    }

    fn disarm(&self) -> io::Result<()> {
        self.set_kill_on_close(false)
    }

    fn set_kill_on_close(&self, enabled: bool) -> io::Result<()> {
        use std::mem::{size_of, zeroed};
        use std::os::windows::io::AsRawHandle;
        use std::ptr::addr_of;
        use windows_sys::Win32::System::JobObjects::{
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JobObjectExtendedLimitInformation, SetInformationJobObject,
        };
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
        if enabled {
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        }
        let ok = unsafe {
            SetInformationJobObject(
                self.handle.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                addr_of!(info).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if ok == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    fn terminate(&self) -> TerminationOutcome {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;
        if unsafe { TerminateJobObject(self.handle.as_raw_handle(), 1) } != 0 {
            TerminationOutcome::Signaled
        } else {
            TerminationOutcome::Failed {
                message: io::Error::last_os_error().to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancellation_is_sticky_across_waiters() {
        let token = CancellationToken::new();
        let waiting = token.clone();
        let task = tokio::spawn(async move { waiting.cancelled().await });
        token.cancel();
        task.await.unwrap();
        token.cancelled().await;
        assert!(token.is_cancelled());
    }

    #[test]
    fn bounded_append_keeps_prefix() {
        let mut bytes = Vec::new();
        let failure = append_limited(&mut bytes, b"abcdef", Some(3), OutputStream::Stdout);
        assert_eq!(bytes, b"abc");
        assert_eq!(
            failure,
            Some(ExecutionFailureKind::CaptureLimit {
                stream: OutputStream::Stdout,
                limit: 3
            })
        );
    }

    #[tokio::test]
    async fn unrepresentable_deadlines_do_not_spawn() {
        use std::process::Stdio;

        let directory = tempfile::tempdir().unwrap();
        for (index, timeout, cleanup_timeout) in [
            (0, Some(Duration::MAX), Duration::from_secs(1)),
            (1, None, Duration::MAX),
        ] {
            let side_effect = directory.path().join(format!("created-{index}"));
            let mut command = Command::new("git");
            command
                .arg("init")
                .arg(&side_effect)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let error = run(
                command,
                None,
                timeout,
                None,
                OutputLimits::default(),
                cleanup_timeout,
            )
            .await
            .unwrap_err();
            assert!(matches!(error, Error::InvalidConfig { .. }));
            assert!(!side_effect.exists());
        }
    }
}
