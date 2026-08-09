#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum RunnerError {
    #[error("I/O error while preparing runner: {0}")]
    Io(#[from] std::io::Error),

    #[error("the vendored runner rootfs is missing: {0}")]
    MissingRootfs(String),

    Container(#[from] hakoniwa::Error),

    #[error("Status code not successful ({}): {}", .0.status.reason, String::from_utf8_lossy(&.0.stderr))]
    NotOk(Box<hakoniwa::Output>),
}
