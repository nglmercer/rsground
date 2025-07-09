use hakoniwa::{Child, ExitStatus};
use nix::sys::wait::{self, WaitPidFlag, WaitStatus};

trait HakoniwaChildExt {
    fn try_wait(&self) -> Option<ExitStatus>;
}

impl HakoniwaChildExt for Child {
    fn try_wait(&self) -> Option<ExitStatus> {
        match wait::waitpid(self.id(), Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => None,
            Ok(WaitStatus::Exited(_, code)) => Some(ExitStatus {
                code,
                // Mario reference
                reason: "Life is good".to_owned(),
                exit_code: None,
                rusage: None,
            }),
            Ok(WaitStatus::Signaled(_, signal, _) | WaitStatus::Stopped(_, signal)) => Some(ExitStatus {
                code: signal as i32,
                reason: signal.as_str().to_owned(),
                exit_code: None,
                rusage: None,
            }),
            Ok(WaitStatus::Continued(_)) => None,
        }
    }
}
