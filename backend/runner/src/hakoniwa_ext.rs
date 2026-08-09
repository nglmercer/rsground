use std::{io::Read, ops, os::fd::AsFd};

use async_io::Async;
use hakoniwa::{Child, ExitStatus};
use nix::{
    sys::wait::{self, WaitPidFlag, WaitStatus},
    unistd::Pid,
};

pub trait HakoniwaChildExt {
    fn try_wait(&self) -> Option<ExitStatus>;
}

impl HakoniwaChildExt for Child {
    fn try_wait(&self) -> Option<ExitStatus> {
        match wait::waitpid(Pid::from_raw(self.id() as i32), Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => None,
            Ok(WaitStatus::Exited(_, code)) => Some(ExitStatus {
                code,
                // Mario reference
                reason: "Life is good".to_owned(),
                exit_code: None,
                rusage: None,
            }),
            Ok(WaitStatus::Signaled(_, signal, _) | WaitStatus::Stopped(_, signal)) => {
                Some(ExitStatus {
                    code: signal as i32,
                    reason: signal.as_str().to_owned(),
                    exit_code: None,
                    rusage: None,
                })
            }
            Ok(WaitStatus::Continued(_)) => None,
            Ok(_) => None,
            Err(err) => {
                println!("[ERROR] {err}");
                None
            }
        }
    }
}

pub struct AsyncOsReader(Async<os_pipe::PipeReader>);

impl From<os_pipe::PipeReader> for AsyncOsReader {
    fn from(value: os_pipe::PipeReader) -> Self {
        Self(Async::new(value).expect("Cannot create async wrapper"))
    }
}

impl AsFd for AsyncOsReader {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsyncOsReader {
    pub async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        _ = self.0.readable().await?;
        unsafe { self.0.get_mut() }.read(buf)
    }

    pub async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let mut total_bytes = 0;
        let mut chunk = [0; 8192];

        loop {
            let read_bytes = self.read(&mut chunk).await?;

            if read_bytes == 0 {
                break;
            }

            buf.extend_from_slice(&chunk[..read_bytes]);
            total_bytes += read_bytes;
        }

        Ok(total_bytes)
    }
}

impl ops::Deref for AsyncOsReader {
    type Target = os_pipe::PipeReader;

    fn deref(&self) -> &Self::Target {
        &self.0.get_ref()
    }
}
