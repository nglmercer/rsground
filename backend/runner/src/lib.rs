pub mod error;
pub mod hakoniwa_ext;

use error::RunnerError;
use hakoniwa::{Child, Command, Container, ExitStatus, Output};
use hakoniwa_ext::AsyncOsReader;
use std::future::Future;
pub use std::io::{PipeReader, PipeWriter};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::{fs, io};

pub const BASE_ENV: [(&str, &str); 3] = [
    ("HOME", "/home"),
    ("PATH", "/bin"),
    ("LD_LIBRARY_PATH", "/lib:/lib64:/libexec"),
];

const VENDORED_ROOTFS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/lxc_rootfs");
const COMMAND_WALL_TIME_SECS: u64 = 60;

pub struct Runner {
    container: Container,
    temp_home: PathBuf,
    host_fallback: bool,
}

impl Runner {
    async fn create_home() -> io::Result<PathBuf> {
        let mut temp_home = PathBuf::from("/tmp");
        temp_home.push(uuid::Uuid::new_v4().simple().to_string());

        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        builder.mode(0o700);
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
            ("/home", FsAccess::R | FsAccess::W | FsAccess::X, true),
            ("/tmp", FsAccess::R | FsAccess::W | FsAccess::X, true),
            ("/dev", FsAccess::R | FsAccess::W, true),
            ("/proc", FsAccess::R, true),
            ("/bin", FsAccess::R | FsAccess::X, false),
            ("/etc", FsAccess::R, false),
            ("/lib", FsAccess::R | FsAccess::X, false),
            ("/lib64", FsAccess::R | FsAccess::X, false),
            ("/usr", FsAccess::R | FsAccess::X, false),
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
            .hostname("rsground")
            // Submitted programs must not be able to reach the host network
            // or internal services. The vendored toolchain is self-contained.
            .unshare(hakoniwa::Namespace::Network);

        if host_fallback {
            container.rootfs("/")?;

            if let Some(host_home) = std::env::var_os("HOME").map(PathBuf::from) {
                let rustup_home = host_home.join(".rustup");
                if rustup_home.is_dir() {
                    container.bindmount_ro(rustup_home.to_string_lossy().as_ref(), "/home/.rustup");
                }

                let cargo_home = host_home.join(".cargo");
                if cargo_home.is_dir() {
                    container.bindmount_rw(cargo_home.to_string_lossy().as_ref(), "/home/.cargo");
                }
            }
        } else {
            container.rootfs(VENDORED_ROOTFS)?;
        }

        Self::configure_filesystem_policy(
            &mut container,
            if host_fallback {
                Path::new("/")
            } else {
                Path::new(VENDORED_ROOTFS)
            },
        );

        container
            .tmpfsmount("/tmp")
            .devfsmount("/dev")
            .procfsmount("/proc")
            .uidmap(1001)
            .gidmap(100)
            .bindmount_rw(temp_home, "/home")
            // Keep compilation and execution bounded even when a submitted
            // program forks, allocates, or writes indefinitely.
            .setrlimit(
                hakoniwa::Rlimit::As,
                4 * 1024 * 1024 * 1024,
                4 * 1024 * 1024 * 1024,
            )
            .setrlimit(hakoniwa::Rlimit::Cpu, 30, 30)
            .setrlimit(hakoniwa::Rlimit::Core, 0, 0)
            .setrlimit(hakoniwa::Rlimit::Fsize, 64 * 1024 * 1024, 64 * 1024 * 1024)
            .setrlimit(hakoniwa::Rlimit::Nofile, 1024, 1024)
            .setrlimit(hakoniwa::Rlimit::Nproc, 1024, 1024);

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
                .env("RUSTUP_HOME", "/home/.rustup")
                .env("CARGO_HOME", "/home/.cargo");
        }
    }

    fn relative_home_path(&self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
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

        Ok(self.temp_home.join(path))
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
        cmd.wait_timeout(COMMAND_WALL_TIME_SECS);

        let mut child = cmd
            .envs(BASE_ENV)
            .current_dir("/home")
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
                let mut status_check_interval = tokio::time::interval(Duration::from_millis(100));

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
                                code: 137,
                                reason: "Aborted".to_owned(),
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
                code: 126,
                reason: "Cannot retrieve exit status".to_owned(),
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
            .current_dir("/home")
            .spawn()
    }

    pub fn start_rls(&mut self) -> hakoniwa::Result<(Child, PipeWriter, PipeReader, PipeReader)> {
        let mut child = self.container.command(if self.host_fallback {
            "/usr/lib/rustup/bin/rust-analyzer"
        } else {
            "/bin/rust-analyzer"
        });
        self.configure_command(&mut child);
        let mut child = child
            .stdin(hakoniwa::Stdio::MakePipe)
            .stdout(hakoniwa::Stdio::MakePipe)
            .stderr(hakoniwa::Stdio::MakePipe)
            .current_dir("/home")
            .spawn()?;

        let stdin = child.stdin.take().expect("Needs communication >:(");
        let stdout = child.stdout.take().expect("Needs communication >:(");
        let stderr = child.stderr.take().expect("Needs communication >:(");

        Ok((child, stdin, stdout, stderr))
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

        let mut cmd = self.container.command("/bin/bash");
        cmd.arg("-c").arg(&arg);
        self.configure_command(&mut cmd);
        cmd
    }

    pub fn cmd_rustc(&self, args: impl IntoIterator<Item = impl AsRef<str>>) -> Command {
        let mut cmd = self.container.command("/bin/rustc");
        // -C linker=/bin/ld -C link-args=-L/lib -C link-args=-L/lib/gcc/x86_64-unknown-linux-gnu/14.2.1
        cmd.args(args);
        self.configure_command(&mut cmd);
        cmd
    }

    pub async fn patch_binary(&self, path: impl AsRef<str>) -> Result<Output, hakoniwa::Error> {
        let path = path.as_ref();
        if !path.starts_with("/home/") || path.contains("..") {
            return Err(hakoniwa::Error::UnError(
                "binary path must be inside /home".to_owned(),
            ));
        }

        let patcher = if self.host_fallback {
            ["/bin/patchelf", "/usr/bin/patchelf"]
                .into_iter()
                .find(|path| Path::new(path).is_file())
        } else {
            Some("/bin/patchelf")
        };

        let Some(patcher) = patcher else {
            // Host toolchains already emit binaries with the host dynamic
            // loader, so no patch is needed in development mode.
            return Ok(Output {
                status: ExitStatus {
                    code: 0,
                    reason: "No binary patching required".to_owned(),
                    exit_code: Some(0),
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
            .arg("/lib/ld-linux-x86-64.so.2")
            .arg(path);
        self.configure_command(&mut command);

        Self::collect_output(&mut command).await
    }
}

fn is_production() -> bool {
    std::env::var("RSGROUND_ENV").is_ok_and(|value| {
        value.eq_ignore_ascii_case("production") || value.eq_ignore_ascii_case("prod")
    })
}

impl Drop for Runner {
    fn drop(&mut self) {
        _ = std::fs::remove_dir_all(&self.temp_home)
            .inspect_err(|err| eprintln!("cannot delete temp home: {err}"));
    }
}
