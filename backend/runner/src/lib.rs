pub mod constants;
pub mod error;
pub mod hakoniwa_ext;

use error::RunnerError;
pub use hakoniwa::Child;
use hakoniwa::{Command, Container, ExitStatus, Output};
use hakoniwa_ext::AsyncOsReader;
use std::future::Future;
pub use std::io::{PipeReader, PipeWriter};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::{fs, io};

use constants as runner_constants;

pub const BASE_ENV: [(&str, &str); 3] = [
    (runner_constants::ENV_HOME, runner_constants::CONTAINER_HOME),
    (runner_constants::ENV_PATH, runner_constants::CONTAINER_BIN),
    (
        runner_constants::ENV_LIBRARY_PATH,
        runner_constants::LIBRARY_PATH,
    ),
];

const VENDORED_ROOTFS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/lxc_rootfs");

pub struct Runner {
    container: Container,
    temp_home: PathBuf,
    host_fallback: bool,
}

impl Runner {
    async fn create_home() -> io::Result<PathBuf> {
        let mut temp_home = PathBuf::from(runner_constants::CONTAINER_TEMP);
        temp_home.push(uuid::Uuid::new_v4().simple().to_string());

        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        builder.mode(runner_constants::TEMP_HOME_MODE);
        builder.create(&temp_home).await?;

        Ok(temp_home)
    }

    fn configure_filesystem_policy(container: &mut Container, rootfs: &Path) {
        use hakoniwa::landlock::{CompatMode, FsAccess, Resource, Ruleset};

        let mut policy = Ruleset::default();
        policy.restrict(Resource::FS, CompatMode::Enforce);

        // The root filesystem is mounted read-only by hakoniwa. Only the
        // per-project home and temporary directories need write/execute
        // access; toolchain and system paths are read/execute only.
        let paths = [
            (
                runner_constants::CONTAINER_HOME,
                FsAccess::R | FsAccess::W | FsAccess::X,
                true,
            ),
            (
                runner_constants::CONTAINER_TEMP,
                FsAccess::R | FsAccess::W | FsAccess::X,
                true,
            ),
            (
                runner_constants::CONTAINER_DEV,
                FsAccess::R | FsAccess::W,
                true,
            ),
            (runner_constants::CONTAINER_PROC, FsAccess::R, true),
            (
                runner_constants::CONTAINER_BIN,
                FsAccess::R | FsAccess::X,
                false,
            ),
            (runner_constants::CONTAINER_ETC, FsAccess::R, false),
            (
                runner_constants::CONTAINER_LIB,
                FsAccess::R | FsAccess::X,
                false,
            ),
            (
                runner_constants::CONTAINER_LIB64,
                FsAccess::R | FsAccess::X,
                false,
            ),
            (
                runner_constants::CONTAINER_USR,
                FsAccess::R | FsAccess::X,
                false,
            ),
        ];

        for (path, access, mounted) in paths {
            if mounted || rootfs.join(path.trim_start_matches('/')).exists() {
                policy.allow_path(path, access);
            }
        }

        container.landlock_ruleset(policy);
    }

    fn create_container(temp_home: &str, host_fallback: bool) -> Result<Container, RunnerError> {
        let mut container = Container::new();
        container
            .hostname(runner_constants::CONTAINER_HOSTNAME)
            // Submitted programs must not be able to reach the host network
            // or internal services. The vendored toolchain is self-contained.
            .unshare(hakoniwa::Namespace::Network);

        if host_fallback {
            container.rootfs(runner_constants::HOST_ROOTFS)?;

            if let Some(host_home) = std::env::var_os(runner_constants::ENV_HOME).map(PathBuf::from)
            {
                let rustup_home = host_home.join(".rustup");
                if rustup_home.is_dir() {
                    container.bindmount_ro(
                        rustup_home.to_string_lossy().as_ref(),
                        runner_constants::RUSTUP_HOME,
                    );
                }

                let cargo_home = host_home.join(".cargo");
                if cargo_home.is_dir() {
                    container.bindmount_rw(
                        cargo_home.to_string_lossy().as_ref(),
                        runner_constants::CARGO_HOME,
                    );
                }
            }
        } else {
            container.rootfs(VENDORED_ROOTFS)?;
        }

        Self::configure_filesystem_policy(
            &mut container,
            if host_fallback {
                Path::new(runner_constants::HOST_ROOTFS)
            } else {
                Path::new(VENDORED_ROOTFS)
            },
        );

        container
            .tmpfsmount(runner_constants::CONTAINER_TEMP)
            .devfsmount(runner_constants::CONTAINER_DEV)
            .procfsmount(runner_constants::CONTAINER_PROC)
            .uidmap(runner_constants::CONTAINER_UID)
            .gidmap(runner_constants::CONTAINER_GID)
            .bindmount_rw(temp_home, runner_constants::CONTAINER_HOME)
            // Keep compilation and execution bounded even when a submitted
            // program forks, allocates, or writes indefinitely.
            .setrlimit(
                hakoniwa::Rlimit::As,
                runner_constants::MEMORY_LIMIT_BYTES,
                runner_constants::MEMORY_LIMIT_BYTES,
            )
            .setrlimit(
                hakoniwa::Rlimit::Cpu,
                runner_constants::CPU_LIMIT_SECS,
                runner_constants::CPU_LIMIT_SECS,
            )
            .setrlimit(
                hakoniwa::Rlimit::Core,
                runner_constants::DISABLED_RLIMIT,
                runner_constants::DISABLED_RLIMIT,
            )
            .setrlimit(
                hakoniwa::Rlimit::Fsize,
                runner_constants::FILE_SIZE_LIMIT_BYTES,
                runner_constants::FILE_SIZE_LIMIT_BYTES,
            )
            .setrlimit(
                hakoniwa::Rlimit::Nofile,
                runner_constants::OPEN_FILE_LIMIT,
                runner_constants::OPEN_FILE_LIMIT,
            )
            .setrlimit(
                hakoniwa::Rlimit::Nproc,
                runner_constants::PROCESS_LIMIT,
                runner_constants::PROCESS_LIMIT,
            );

        Ok(container)
    }

    pub fn validate_environment() -> Result<(), RunnerError> {
        let host_fallback = !Path::new(VENDORED_ROOTFS).is_dir();

        // A host-rootfs fallback is acceptable only for local debug builds;
        // release binaries must never expose the host filesystem to jobs.
        if host_fallback && (is_production() || !cfg!(debug_assertions)) {
            return Err(RunnerError::MissingRootfs(VENDORED_ROOTFS.to_owned()));
        }

        Ok(())
    }

    pub async fn new() -> Result<Self, RunnerError> {
        Self::validate_environment()?;
        let host_fallback = !Path::new(VENDORED_ROOTFS).is_dir();

        let temp_home = Self::create_home().await?;

        if host_fallback {
            log::warn!(
                "Runner rootfs is missing at {VENDORED_ROOTFS}; using the host rootfs fallback for local development. Install the vendored rootfs before deployment."
            );
        }

        let temp_home_str = temp_home
            .to_str()
            .ok_or_else(|| RunnerError::MissingRootfs(temp_home.display().to_string()))?;
        let container = match Self::create_container(temp_home_str, host_fallback) {
            Ok(container) => container,
            Err(error) => {
                _ = fs::remove_dir_all(&temp_home).await;
                return Err(error);
            }
        };

        Ok(Self {
            container,
            temp_home,
            host_fallback,
        })
    }

    fn configure_command(&self, command: &mut Command) {
        command.envs(BASE_ENV);

        if self.host_fallback {
            // The development host uses rustup proxies. Their toolchains and
            // registry are mounted at these stable container paths above.
            command
                .env(
                    runner_constants::ENV_RUSTUP_HOME,
                    runner_constants::RUSTUP_HOME,
                )
                .env(
                    runner_constants::ENV_CARGO_HOME,
                    runner_constants::CARGO_HOME,
                );
        }
    }

    fn relative_home_path(&self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
        relative_home_path(&self.temp_home, path)
    }

    pub async fn create_file(
        &self,
        container_path: impl AsRef<str>,
        content: impl AsRef<str>,
    ) -> io::Result<()> {
        let home = self.relative_home_path(container_path.as_ref())?;

        if let Some(parent) = home.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(home, content.as_ref()).await
    }

    pub async fn remove_file(&self, container_path: impl AsRef<str>) -> io::Result<()> {
        let path = self.relative_home_path(container_path.as_ref())?;
        match fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub async fn copy_file_from_runner(
        &self,
        other: &Runner,
        host_path: impl AsRef<Path>,
        other_path: impl AsRef<Path>,
    ) -> io::Result<()> {
        let host_file_path = self.relative_home_path(host_path)?;
        let other_file_path = other.relative_home_path(other_path)?;

        if let Some(parent) = host_file_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::copy(other_file_path, host_file_path).await.map(|_| ())
    }

    pub async fn collect_output(cmd: &mut Command) -> Result<Output, hakoniwa::Error> {
        async fn collect(stream: Option<AsyncOsReader>) -> Vec<u8> {
            let mut buf = Vec::new();

            let Some(mut stream) = stream else { return buf };

            _ = stream.read_to_end(&mut buf).await;

            buf
        }

        Self::stream_output(cmd, collect, collect, None)
            .await
            .map(|(status, stdout, stderr)| Output {
                status,
                stdout,
                stderr,
            })
    }

    pub async fn stream_output<Stdout, Stderr, StdoutAsync, StderrAsync, StdoutFn, StderrFn>(
        cmd: &mut Command,
        stdout_fn: StdoutFn,
        stderr_fn: StderrFn,
        abort: Option<oneshot::Receiver<()>>,
    ) -> Result<(ExitStatus, Stdout, Stderr), hakoniwa::Error>
    where
        Stdout: Default + Send + 'static,
        StdoutAsync: Future<Output = Stdout> + Send + 'static,
        StdoutFn: FnOnce(Option<AsyncOsReader>) -> StdoutAsync,
        Stderr: Default + Send + 'static,
        StderrAsync: Future<Output = Stderr> + Send + 'static,
        StderrFn: FnOnce(Option<AsyncOsReader>) -> StderrAsync,
    {
        cmd.wait_timeout(runner_constants::COMMAND_WALL_TIME_SECS);

        let mut child = cmd
            .envs(BASE_ENV)
            .current_dir(runner_constants::CONTAINER_HOME)
            .stdin(hakoniwa::Stdio::MakePipe)
            .stdout(hakoniwa::Stdio::MakePipe)
            .stderr(hakoniwa::Stdio::MakePipe)
            .spawn()?;

        let stdout = child.stdout.take().map(AsyncOsReader::from);
        let stdout = tokio::spawn(stdout_fn(stdout));
        let stderr = child.stderr.take().map(AsyncOsReader::from);
        let stderr = tokio::spawn(stderr_fn(stderr));

        let status = if let Some(mut abort) = abort {
            tokio::spawn(async move {
                let mut status_check_interval = tokio::time::interval(Duration::from_millis(
                    runner_constants::STATUS_POLL_INTERVAL_MS,
                ));

                loop {
                    tokio::select! {
                        _ = status_check_interval.tick() => {
                            if let Some(status) = child.try_wait()? {
                                return Ok(status)
                            }
                        }
                        _ = &mut abort => {
                            _ = child.kill();
                            _ = child.wait();
                            return Ok(ExitStatus {
                                code: runner_constants::OUTPUT_ABORTED_CODE,
                                reason: runner_constants::OUTPUT_ABORTED_REASON.to_owned(),
                                exit_code: None,
                                rusage: None,
                                proc_pid_smaps_rollup: None,
                                proc_pid_status: None,
                            });
                        },
                    }
                }
            })
        } else {
            tokio::task::spawn_blocking(move || child.wait())
        };

        let (status, stdout, stderr) = tokio::join!(status, stdout, stderr);

        let status = status
            .inspect_err(|err| eprintln!("Join error: {err}"))
            .map(|o| o.inspect_err(|err| eprintln!("Join error: {err}")).ok())
            .ok()
            .flatten()
            .unwrap_or(ExitStatus {
                code: runner_constants::OUTPUT_UNAVAILABLE_CODE,
                reason: runner_constants::OUTPUT_UNAVAILABLE_REASON.to_owned(),
                exit_code: None,
                rusage: None,
                proc_pid_smaps_rollup: None,
                proc_pid_status: None,
            });

        let stdout = stdout
            .inspect_err(|err| eprintln!("Join error: {err}"))
            .unwrap_or_default();

        let stderr = stderr
            .inspect_err(|err| eprintln!("Join error: {err}"))
            .unwrap_or_default();

        Ok((status, stdout, stderr))
    }

    /// Spawn process with shared stdio.
    /// Focused in interactive shell for manual testing
    pub fn spawn(
        &self,
        cmd: impl AsRef<str>,
        args: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<hakoniwa::Child, hakoniwa::Error> {
        let mut command = self.container.command(cmd.as_ref());
        command.args(args);
        self.configure_command(&mut command);
        command
            .stdin(hakoniwa::Stdio::Inherit)
            .stdout(hakoniwa::Stdio::Inherit)
            .stderr(hakoniwa::Stdio::Inherit)
            .current_dir(runner_constants::CONTAINER_HOME)
            .spawn()
    }

    /// Start the Rust Analyzer language server inside this runner's sandbox.
    ///
    /// The old `start_rls` name is kept as a compatibility wrapper because
    /// the runner tests and downstream callers used it before the project
    /// switched to Rust Analyzer.
    pub fn start_rust_analyzer(
        &self,
    ) -> hakoniwa::Result<(Child, PipeWriter, PipeReader, PipeReader)> {
        let mut child = self.container.command(if self.host_fallback {
            runner_constants::HOST_RUST_ANALYZER
        } else {
            runner_constants::RUST_ANALYZER
        });
        self.configure_command(&mut child);
        let mut child = child
            .stdin(hakoniwa::Stdio::MakePipe)
            .stdout(hakoniwa::Stdio::MakePipe)
            .stderr(hakoniwa::Stdio::MakePipe)
            .current_dir(runner_constants::CONTAINER_HOME)
            .spawn()?;

        let stdin = child.stdin.take().expect("Needs communication >:(");
        let stdout = child.stdout.take().expect("Needs communication >:(");
        let stderr = child.stderr.take().expect("Needs communication >:(");

        Ok((child, stdin, stdout, stderr))
    }

    pub fn start_rls(&self) -> hakoniwa::Result<(Child, PipeWriter, PipeReader, PipeReader)> {
        self.start_rust_analyzer()
    }

    pub fn cmd(
        &self,
        cmd: impl AsRef<str>,
        args: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Command {
        let mut cmd = self.container.command(cmd.as_ref());
        cmd.args(args);
        self.configure_command(&mut cmd);
        cmd
    }

    pub fn cmd_bash(
        &self,
        cmd: impl AsRef<str>,
        args: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Command {
        let args = args
            .into_iter()
            .map(|a| format!("{:?}", a.as_ref()))
            .collect::<Vec<_>>()
            .join(" ");
        let arg = format!("{} {args}", cmd.as_ref());

        let mut cmd = self.container.command(runner_constants::BASH);
        cmd.arg("-c").arg(&arg);
        self.configure_command(&mut cmd);
        cmd
    }

    pub fn cmd_rustc(&self, args: impl IntoIterator<Item = impl AsRef<str>>) -> Command {
        let mut cmd = self.container.command(runner_constants::RUSTC);
        // -C linker=/bin/ld -C link-args=-L/lib -C link-args=-L/lib/gcc/x86_64-unknown-linux-gnu/14.2.1
        cmd.args(args);
        self.configure_command(&mut cmd);
        cmd
    }

    pub async fn patch_binary(&self, path: impl AsRef<str>) -> Result<Output, hakoniwa::Error> {
        let path = path.as_ref();
        validate_binary_path(path)?;

        let patcher = if self.host_fallback {
            [runner_constants::PATCHELF, runner_constants::HOST_PATCHELF]
                .into_iter()
                .find(|path| Path::new(path).is_file())
        } else {
            Some(runner_constants::PATCHELF)
        };

        let Some(patcher) = patcher else {
            // Host toolchains already emit binaries with the host dynamic
            // loader, so no patch is needed in development mode.
            return Ok(Output {
                status: ExitStatus {
                    code: runner_constants::SUCCESS_EXIT_CODE,
                    reason: runner_constants::NO_PATCH_REASON.to_owned(),
                    exit_code: Some(runner_constants::SUCCESS_EXIT_CODE),
                    rusage: None,
                    proc_pid_smaps_rollup: None,
                    proc_pid_status: None,
                },
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        };

        let mut command = self.container.command(patcher);
        command
            .arg("--set-interpreter")
            .arg(runner_constants::DYNAMIC_LOADER)
            .arg(path);
        self.configure_command(&mut command);

        Self::collect_output(&mut command).await
    }
}

fn is_production() -> bool {
    std::env::var(runner_constants::ENVIRONMENT).is_ok_and(|value| {
        value.eq_ignore_ascii_case(runner_constants::PRODUCTION)
            || value.eq_ignore_ascii_case(runner_constants::PRODUCTION_ALIAS)
    })
}

fn relative_home_path(base: &Path, path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let path = path.as_ref();

    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "runner paths must be relative and must not contain '..'",
        ));
    }

    Ok(base.join(path))
}

fn validate_binary_path(path: &str) -> Result<(), hakoniwa::Error> {
    if !path.starts_with(runner_constants::HOME_PATH_PREFIX) || path.contains("..") {
        return Err(hakoniwa::Error::UnError(
            runner_constants::INVALID_BINARY_PATH.to_owned(),
        ));
    }

    Ok(())
}

impl Drop for Runner {
    fn drop(&mut self) {
        _ = std::fs::remove_dir_all(&self.temp_home)
            .inspect_err(|err| eprintln!("cannot delete temp home: {err}"));
    }
}

#[cfg(test)]
mod tests {
    use super::{relative_home_path, validate_binary_path};
    use std::path::Path;

    #[test]
    fn confines_runner_paths_to_the_project_home() {
        let base = Path::new("/tmp/rsground-runner-home");
        assert_eq!(
            relative_home_path(base, "src/main.rs").unwrap(),
            base.join("src/main.rs")
        );
        assert_eq!(
            relative_home_path(base, "./src/main.rs").unwrap(),
            base.join("./src/main.rs")
        );
    }

    #[test]
    fn rejects_empty_absolute_and_parent_paths() {
        let base = Path::new("/tmp/rsground-runner-home");
        for path in ["", "/etc/passwd", "../outside", "src/../../outside"] {
            assert!(
                relative_home_path(base, path).is_err(),
                "path should be rejected: {path:?}"
            );
        }
    }

    #[test]
    fn validates_binary_paths_before_starting_the_patcher() {
        assert!(validate_binary_path("/home/main").is_ok());
        assert!(validate_binary_path("/home/project/bin").is_ok());
        assert!(validate_binary_path("main").is_err());
        assert!(validate_binary_path("/tmp/main").is_err());
        assert!(validate_binary_path("/home/../main").is_err());
    }
}
