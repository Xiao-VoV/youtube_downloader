//! Concurrent bounded readers and retention, independent of lifecycle.
use crate::{EngineConfig, EngineError, ErrorKind, Result};
use std::{io::Read, process::Child, sync::mpsc, thread};
#[derive(Default)]
pub(crate) struct Output {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
pub(super) enum Chunk {
    Data(bool, Vec<u8>),
    End,
    Error,
}

pub(super) fn start_readers(
    child: &mut Child,
) -> (mpsc::Receiver<Chunk>, Vec<thread::JoinHandle<()>>) {
    let (tx, rx) = mpsc::sync_channel(64);
    let pipes: Vec<(bool, Box<dyn Read + Send>)> = vec![
        (true, Box::new(child.stdout.take().expect("piped stdout"))),
        (false, Box::new(child.stderr.take().expect("piped stderr"))),
    ];
    let readers = pipes
        .into_iter()
        .map(|(stdout, mut pipe)| {
            let tx = tx.clone();
            thread::spawn(move || {
                let mut buf = [0; 8192];
                loop {
                    match pipe.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            if tx.send(Chunk::Data(stdout, buf[..n].to_vec())).is_err() {
                                return;
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => {
                            let _ = tx.send(Chunk::Error);
                            break;
                        }
                    }
                }
                let _ = tx.send(Chunk::End);
            })
        })
        .collect();
    (rx, readers)
}

pub(super) struct PipeState {
    pub output: Output,
    pub pending: Vec<u8>,
    pub ended: usize,
    capture: bool,
}
impl PipeState {
    pub fn new(capture: bool) -> Self {
        Self {
            output: Output::default(),
            pending: Vec::new(),
            ended: 0,
            capture,
        }
    }
    pub fn consume(
        &mut self,
        chunk: Chunk,
        config: &EngineConfig,
        ignore_stdout: bool,
        line: &mut impl FnMut(&[u8]) -> Result<()>,
    ) -> Result<()> {
        match chunk {
            Chunk::End => self.ended += 1,
            Chunk::Error => return Err(EngineError::new(ErrorKind::Io, "tool pipe read failed")),
            Chunk::Data(false, bytes) => {
                let keep = config
                    .max_stderr_bytes
                    .saturating_sub(self.output.stderr.len())
                    .min(bytes.len());
                self.output.stderr.extend_from_slice(&bytes[..keep]);
            }
            Chunk::Data(true, bytes) if !ignore_stdout => self.stdout(bytes, config, line)?,
            _ => {}
        }
        Ok(())
    }
    fn stdout(
        &mut self,
        bytes: Vec<u8>,
        config: &EngineConfig,
        line: &mut impl FnMut(&[u8]) -> Result<()>,
    ) -> Result<()> {
        if self.capture {
            if self.output.stdout.len().saturating_add(bytes.len()) > config.max_stdout_bytes {
                return Err(EngineError::new(
                    ErrorKind::OutputLimit,
                    "stdout exceeds configured limit",
                ));
            }
            self.output.stdout.extend_from_slice(&bytes);
            return Ok(());
        }
        for b in bytes {
            if b == b'\n' {
                line(&self.pending)?;
                self.pending.clear();
            } else if self.pending.len() >= config.max_line_bytes {
                return Err(EngineError::new(
                    ErrorKind::OutputLimit,
                    "stdout line exceeds configured limit",
                ));
            } else {
                self.pending.push(b);
            }
        }
        Ok(())
    }
}
