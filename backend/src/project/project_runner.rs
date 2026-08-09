use std::sync::{Arc, Mutex};

use actix::{Actor, ActorResponse, Addr, Context, Handler, Message, WrapFuture};
use rsground_runner::{error::RunnerError, Runner};
use tokio::sync::{broadcast, oneshot};
use uuid::Uuid;

use crate::constants::{output, project};
use crate::ws::messages::{OutputChannel, ServerMessage};

pub type AbortNotify = Arc<Mutex<Option<oneshot::Sender<()>>>>;

#[derive(Clone)]
pub struct ProjectExecuter {
    pub project_id: Uuid,
    pub broadcast: broadcast::Sender<ServerMessage>,
    pub runner: Arc<Runner>,
    pub execution: AbortNotify,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Execute;

impl ProjectExecuter {
    pub async fn start(
        project_id: Uuid,
        broadcast: broadcast::Sender<ServerMessage>,
    ) -> Result<(Arc<Runner>, AbortNotify, Addr<Self>), RunnerError> {
        let runner = Arc::new(Runner::new().await?);
        let execution: AbortNotify = Mutex::new(None).into();

        let project_executer = Self {
            project_id,
            broadcast,
            runner: runner.clone(),
            execution: execution.clone(),
        };

        Ok((runner, execution, project_executer.start()))
    }
}

impl Actor for ProjectExecuter {
    type Context = Context<ProjectExecuter>;
}

impl Handler<Execute> for ProjectExecuter {
    type Result = ActorResponse<Self, <Execute as actix::Message>::Result>;

    fn handle(&mut self, _: Execute, _: &mut Self::Context) -> Self::Result {
        let cloned = self.clone();

        ActorResponse::r#async(
            async move {
                _ = execute(cloned).await;
            }
            .into_actor(self),
        )
    }
}

async fn execute(project: ProjectExecuter) -> Result<(), ()> {
    let execution = project.execution.clone();
    let abort = {
        let Ok(mut execution) = execution.lock() else {
            return Err(());
        };

        if execution.is_some() {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel::<()>();
        *execution = Some(tx);
        rx
    };

    let result = execute_inner(project, abort).await;

    if let Ok(mut execution) = execution.lock() {
        *execution = None;
    }

    result
}

async fn execute_inner(project: ProjectExecuter, abort: oneshot::Receiver<()>) -> Result<(), ()> {
    let project_id = project.project_id;
    let broadcast = project.broadcast;
    let runner = project.runner;

    macro_rules! stream {
        ($channel:expr) => {{
            let broadcast = broadcast.clone();

            async move |stdout| {
                let Some(mut stdout) = stdout else { return };

                let buf = &mut [0; output::BUFFER_SIZE];

                loop {
                    let Ok(size) = stdout.read(buf).await else {
                        log::trace!("Cannot read");
                        break;
                    };

                    if size == 0 {
                        break;
                    }

                    // log::trace!(concat!(stringify!($channel), ": {:x?}"), &buf[..size]);
                    _ = broadcast.send(ServerMessage::SyncOutput {
                        channel: $channel,
                        buf: buf[..size].to_vec(),
                    });
                }
            }
        }};
    }

    log::trace!("Execute started for {project_id}");

    _ = broadcast.send(ServerMessage::SyncOutputStart);

    log::trace!("[Execute] compiling in {project_id}");

    let (status, _, _) = Runner::stream_output(
        &mut runner.cmd_rustc([
            project::RUSTC_COLOR_ARGUMENT,
            project::RUSTC_COLOR_ALWAYS,
            project::RUNNER_MAIN_FILE,
        ]),
        stream!(OutputChannel::Stdout),
        stream!(OutputChannel::Stderr),
        Some(abort),
    )
    .await
    .map_err(|err| {
        log::error!("[Execute] compilation failed in {project_id}: {err}");
        _ = broadcast.send(ServerMessage::SyncOutput {
            channel: OutputChannel::Stderr,
            buf: err.to_string().into_bytes(),
        });
        _ = broadcast.send(ServerMessage::SyncOutputEnd {
            exit_code: output::COMPILE_FAILURE_EXIT_CODE,
        });
    })?;

    if !status.success() {
        log::error!("[Execute] compilation failed in {project_id}");
        _ = broadcast.send(ServerMessage::SyncOutputEnd {
            exit_code: status.code.clamp(0, u8::MAX as i32) as u8,
        });

        return Err(());
    }

    log::trace!("[Execute] patching in {project_id}");

    let output = runner
        .patch_binary(project::RUNNER_MAIN_EXECUTABLE)
        .await
        .map_err(|err| {
            log::error!("[Execute] patching failed in {project_id}: {err}");
            _ = broadcast.send(ServerMessage::SyncOutput {
                channel: OutputChannel::Stderr,
                buf: err.to_string().into_bytes(),
            });
            _ = broadcast.send(ServerMessage::SyncOutputEnd {
                exit_code: output::COMPILE_FAILURE_EXIT_CODE,
            });
        })?;

    if !output.status.success() {
        log::error!("[Execute] patch failed in {project_id}: {output:#?}");
        _ = broadcast.send(ServerMessage::SyncOutput {
            channel: OutputChannel::Stdout,
            buf: output.stdout,
        });
        _ = broadcast.send(ServerMessage::SyncOutput {
            channel: OutputChannel::Stderr,
            buf: output.stderr,
        });
        _ = broadcast.send(ServerMessage::SyncOutputEnd {
            exit_code: output::COMPILE_FAILURE_EXIT_CODE,
        });

        return Err(());
    }
    log::trace!("[Execute] running in {project_id}");

    let abort = {
        let (tx, rx) = oneshot::channel::<()>();

        *project.execution.lock().unwrap() = Some(tx);

        rx
    };

    let (exit_code, _, _) = Runner::stream_output(
        &mut runner.cmd(
            project::RUNNER_MAIN_EXECUTABLE,
            rsground_runner::constants::EMPTY_ARGS,
        ),
        stream!(OutputChannel::Stdout),
        stream!(OutputChannel::Stderr),
        Some(abort),
    )
    .await
    .map_err(|err| {
        log::trace!("[Execute] run failed in {project_id}: {err}");
        _ = broadcast.send(ServerMessage::SyncOutput {
            channel: OutputChannel::Stderr,
            buf: err.to_string().into_bytes(),
        });
        _ = broadcast.send(ServerMessage::SyncOutputEnd {
            exit_code: output::COMPILE_FAILURE_EXIT_CODE,
        });
    })?;

    log::trace!("[Execute] finish in {project_id}");

    _ = broadcast.send(ServerMessage::SyncOutputEnd {
        exit_code: exit_code.code.clamp(0, u8::MAX as i32) as u8,
    });

    Ok(())
}
