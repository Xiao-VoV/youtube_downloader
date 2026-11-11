//! Unix process-group lifecycle with bounded concurrent pipe consumption.
pub(crate) use crate::pipes::Output;
use crate::pipes::{start_readers, PipeState};
use crate::{CancelToken, EngineConfig, EngineError, ErrorKind, Result};
use std::{
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

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
    #[cfg(not(unix))]
    return Err(EngineError::new(
        ErrorKind::Unsupported,
        "process-tree supervision currently requires Unix",
    ));
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
