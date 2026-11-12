//! Unix process-group lifecycle with bounded concurrent pipe consumption.
pub(crate) use crate::pipes::Output;
use crate::pipes::{PipeState, start_readers};
use crate::{CancelToken, EngineConfig, EngineError, ErrorKind, Result};
use std::{
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

#[cfg(windows)]
fn windows_run(
    mut command: Command,
    config: &EngineConfig,
    timeout: Duration,
    cancel: &CancelToken,
    capture: bool,
    mut line: impl FnMut(&[u8]) -> Result<()>,
) -> Result<Output> {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::{
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject, TerminateJobObject,
            },
            Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE},
        },
    };
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if job == 0 {
        return Err(EngineError::new(
            ErrorKind::Spawn,
            "cannot create Windows Job Object",
        ));
    }
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
        BasicLimitInformation: Default::default(),
        IoInfo: Default::default(),
        ProcessMemoryLimit: 0,
        JobMemoryLimit: 0,
        PeakProcessMemoryUsed: 0,
        PeakJobMemoryUsed: 0,
    };
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const _,
            std::mem::size_of_val(&limits) as u32,
        )
    } == 0
    {
        unsafe {
            CloseHandle(job);
        }
        return Err(EngineError::new(
            ErrorKind::Spawn,
            "cannot configure Windows Job Object",
        ));
    }
    let mut child = command
        .spawn()
        .map_err(|_| EngineError::new(ErrorKind::Spawn, "unable to start selected tool"))?;
    let process = unsafe { OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, child.id()) };
    if process == 0 || unsafe { AssignProcessToJobObject(job, process) } == 0 {
        unsafe {
            if process != 0 {
                CloseHandle(process);
            }
            TerminateJobObject(job, 1);
            CloseHandle(job);
        }
        return Err(EngineError::new(
            ErrorKind::Spawn,
            "cannot assign tool to Windows Job Object",
        ));
    }
    let (rx, readers) = start_readers(&mut child);
    let started = Instant::now();
    let mut stopped = false;
    let mut cancelled = false;
    let mut timed_out = false;
    let mut pipes = PipeState::new(capture);
    loop {
        if !stopped && cancel.is_cancelled() {
            cancelled = true;
            unsafe {
                TerminateJobObject(job, 1);
            }
            stopped = true;
        } else if !stopped && started.elapsed() >= timeout {
            timed_out = true;
            unsafe {
                TerminateJobObject(job, 1);
            }
            stopped = true;
        }
        if child.try_wait().ok().flatten().is_some() {
            stopped = true;
        }
        if stopped && pipes.ended == 2 {
            break;
        }
        if let Ok(chunk) = rx.recv_timeout(Duration::from_millis(20)) {
            pipes.consume(chunk, config, stopped, &mut line)?;
        }
    }
    let status = child
        .wait()
        .map_err(|_| EngineError::new(ErrorKind::Io, "tool reap failed"))?;
    for reader in readers {
        let _ = reader.join();
    }
    unsafe {
        CloseHandle(process);
        CloseHandle(job);
    }
    if cancelled {
        return Err(EngineError::new(
            ErrorKind::Cancelled,
            "cancelled; job stopped",
        ));
    }
    if timed_out {
        return Err(EngineError::new(
            ErrorKind::Timeout,
            "tool deadline exceeded; job stopped",
        ));
    }
    if !status.success() {
        return Err(EngineError::new(
            ErrorKind::ProcessFailed,
            "tool exited unsuccessfully",
        ));
    }
    if !capture && !pipes.pending.is_empty() {
        line(&pipes.pending)?;
    }
    Ok(pipes.output)
}

#[cfg(unix)]
fn signal_group(pid: u32, sig: i32) {
    // Negative PID addresses the group created at spawn, not only yt-dlp.
    unsafe {
        libc::kill(-(pid as i32), sig);
    }
}

#[cfg(unix)]
struct Lifecycle {
    start: Instant,
    stopping: Option<Instant>,
    killed: bool,
    status: Option<ExitStatus>,
    error: Option<EngineError>,
}
#[cfg(unix)]
impl Lifecycle {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            stopping: None,
            killed: false,
            status: None,
            error: None,
        }
    }
    fn fail(&mut self, error: EngineError) {
        self.error.get_or_insert(error);
    }
    fn tick(
        &mut self,
        child: &mut Child,
        config: &EngineConfig,
        timeout: Duration,
        cancel: &CancelToken,
    ) {
        if self.error.is_none() {
            if cancel.is_cancelled() {
                self.fail(EngineError::new(
                    ErrorKind::Cancelled,
                    "cancelled; process group stopped",
                ));
            } else if self.start.elapsed() >= timeout {
                self.fail(EngineError::new(
                    ErrorKind::Timeout,
                    "tool deadline exceeded; process group stopped",
                ));
            }
        }
        if self.status.is_none() {
            match child.try_wait() {
                Ok(status) => self.status = status,
                Err(_) => self.fail(EngineError::new(ErrorKind::Io, "cannot wait for tool")),
            }
        }
        // Successful root exit also starts cleanup immediately. A postprocessor
        // cannot hold the pipe open until the overall download timeout expires.
        if (self.error.is_some() || self.status.is_some()) && self.stopping.is_none() {
            signal_group(child.id(), libc::SIGTERM);
            self.stopping = Some(Instant::now());
        }
        if self
            .stopping
            .is_some_and(|at| at.elapsed() >= config.terminate_grace)
            && !self.killed
        {
            signal_group(child.id(), libc::SIGKILL);
            self.killed = true;
        }
    }
}

/// Run with pipe backpressure bounded independently from retained output.
/// This is process supervision, not an OS sandbox: trusted installed tools must
/// not escape the process group with setsid or pass pipes to unrelated processes.
pub(crate) fn run(
    mut command: Command,
    config: &EngineConfig,
    timeout: Duration,
    cancel: &CancelToken,
    capture: bool,
    mut line: impl FnMut(&[u8]) -> Result<()>,
) -> Result<Output> {
    if cancel.is_cancelled() {
        return Err(EngineError::new(
            ErrorKind::Cancelled,
            "cancelled before spawn",
        ));
    }
    #[cfg(all(not(unix), not(windows)))]
    return Err(EngineError::new(
        ErrorKind::Unsupported,
        "process-tree supervision is unavailable on this platform",
    ));
    #[cfg(windows)]
    return windows_run(command, config, timeout, cancel, capture, line);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|_| EngineError::new(ErrorKind::Spawn, "unable to start selected tool"))?;
        let (rx, readers) = start_readers(&mut child);
        let mut life = Lifecycle::new();
        let mut pipes = PipeState::new(capture);
        loop {
            life.tick(&mut child, config, timeout, cancel);
            if life.status.is_some() && pipes.ended == 2 && life.killed {
                break;
            }
            if let Ok(chunk) = rx.recv_timeout(Duration::from_millis(20)) {
                if let Err(error) = pipes.consume(chunk, config, life.error.is_some(), &mut line) {
                    life.fail(error);
                }
            }
        }
        let waited = child.wait();
        for reader in readers {
            let _ = reader.join();
        }
        if let Some(error) = life.error {
            return Err(error);
        }
        if !capture && !pipes.pending.is_empty() {
            line(&pipes.pending)?;
        }
        let status = waited.map_err(|_| EngineError::new(ErrorKind::Io, "tool reap failed"))?;
        if !status.success() {
            return Err(EngineError::new(
                ErrorKind::ProcessFailed,
                format!(
                    "tool exited unsuccessfully ({:?}); diagnostics retained privately",
                    status.code()
                ),
            ));
        }
        Ok(pipes.output)
    }
}
