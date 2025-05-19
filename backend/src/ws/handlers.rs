use std::io::Read;

use actix_ws as ws;
use rsground_runner::Runner;

use crate::collab::Document;
use crate::ws::messages::{ClientMessage, ServerMessage, ServerMessageError};
use crate::ws::ws_ext::SessionExt;

use super::messages::OutputChannel;
use super::websocket::RgWebsocket;

impl RgWebsocket {
    async fn handle_ws_response(
        ctx: &mut ws::Session,
        msg: Result<ServerMessage, ServerMessageError>,
    ) {
        let response = match msg {
            Ok(ok) => ok,
            Err(ServerMessageError::None) => return,
            Err(err) => err.into(),
        };

        log::trace!("Sending response: {response:#?}");
        _ = ctx.text_json(&response).await;
    }

    pub async fn handle_welcome(&self, ctx: &mut ws::Session) {
        // Don't send welcome when is in queue
        if !self.access.can_read() {
            return;
        }

        Self::handle_ws_response(ctx, self.compose_welcome().await).await
    }

    async fn compose_welcome(&self) -> Result<ServerMessage, ServerMessageError> {
        let mut manager = self.app_state.get_manager();
        let project = manager.get_project_mut(self.project_id)?;

        _ = project.broadcast.send(ServerMessage::UserConnected {
            user_id: self.user_info.id.clone(),
            user_name: self.user_info.name.clone(),
        });

        let files = project.get_files().clone();
        let users = project
            .allowed_users
            .iter()
            .filter_map(|(user, access)| {
                self.app_state
                    .get_username(&user)
                    .map(|username| (user.clone(), (username, *access)))
            })
            .collect();

        let requests = if project.owner == self.user_info.id {
            Some(
                project
                    .requests
                    .iter()
                    .filter_map(|user| {
                        self.app_state
                            .get_username(&user)
                            .map(|username| (user.clone(), username))
                    })
                    .collect(),
            )
        } else {
            None
        };

        Ok(ServerMessage::Welcome {
            session_id: self.session_id.clone(),
            files,
            users,
            requests,
        })
    }

    pub async fn handle_broadcast(&mut self, msg: ServerMessage, ctx: &mut ws::Session) {
        match msg {
            ServerMessage::UpdateAccess { access, user_id } if user_id == self.user_info.id => {
                self.access = access;

                _ = ctx
                    .text_json(&ServerMessage::UpdateAccess { access, user_id })
                    .await;
            }
            // Only update requests to owner
            ServerMessage::RequestAccess { .. }
                if self
                    .app_state
                    .get_manager()
                    .get_project(&self.project_id)
                    .is_some_and(|p| p.owner == self.user_info.id) =>
            {
                _ = ctx.text_json(&msg).await
            }
            ServerMessage::RequestAccess { .. } => {}

            _ if self.access.can_read() => _ = ctx.text_json(&msg).await,
            _ => {}
        }
    }

    pub async fn handle_ws_msg(
        &mut self,
        msg: Result<ws::AggregatedMessage, ws::ProtocolError>,
        ctx: &mut ws::Session,
    ) {
        let Ok(msg) = msg.inspect_err(|e| log::error!("Error in websocket stream: {e:?}")) else {
            return;
        };

        match msg {
            ws::AggregatedMessage::Text(text) => {
                if text == "ping" {
                    return;
                }

                log::trace!("New message: {text}");

                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        let msg = self.handle_client_message(&ctx, client_msg).await;

                        Self::handle_ws_response(ctx, msg).await;
                    }
                    Err(err) => {
                        log::error!("Could not parse message: {err}");

                        let err = ServerMessage::Error {
                            message: err.to_string(),
                        };
                        _ = ctx.text_json(&err).await;
                    }
                }
            }
            ws::AggregatedMessage::Close(reason) => {
                log::info!("Closed connection: {reason:?}");
                _ = ctx.clone().close(reason).await;
            }
            _ => (),
        }
    }

    async fn handle_client_message(
        &mut self,
        _ctx: &ws::Session,
        msg: ClientMessage,
    ) -> Result<ServerMessage, ServerMessageError> {
        self.access.need_read()?;

        let mut manager = self.app_state.get_manager();

        match msg {
            ClientMessage::Config {
                name,
                is_public,
                password,
            } => {
                let project = manager.get_project_mut(self.project_id)?;

                if project.owner != self.user_info.id {
                    return Err(ServerMessageError::None);
                }

                if let Some(name) = name {
                    project.name = name;
                }

                if let Some(is_public) = is_public {
                    project.is_public = is_public;
                }

                if let Some(password) = password {
                    if password.is_empty() {
                        project.password = None;
                    } else {
                        project.password = project.is_public.then_some(password);
                    }
                }

                _ = project.broadcast.send(ServerMessage::ProjectConfig {
                    name: project.name.clone(),
                    is_public: project.is_public,
                    password: project.password.clone(),
                });

                Err(ServerMessageError::None)
            }
            ClientMessage::Execute => {
                self.access.need_editor()?;

                let project = manager.get_project_mut(self.project_id)?;
                let broadcast = project.broadcast.clone();

                let runner = project.get_runner().await.clone();

                macro_rules! stream {
                    ($broadcast:expr, $channel:expr) => {{
                        let broadcast = $broadcast;

                        async move |stdout| {
                            let Some(mut stdout) = stdout else { return };

                            let buf = &mut [0; 2048];

                            loop {
                                let Ok(size) = stdout.read(buf) else {
                                    log::trace!("Cannot read");
                                    break;
                                };

                                if size == 0 {
                                    break;
                                }

                                _ = broadcast.send(ServerMessage::SyncOutput {
                                    channel: $channel,
                                    buf: buf[..size].to_vec(),
                                });
                            }
                        }
                    }};
                }

                let project_id = self.project_id;

                actix::spawn(async move {
                    log::trace!("Execute started for {project_id}");

                    _ = broadcast.send(ServerMessage::SyncOutputStart);

                    log::trace!("[Execute] compiling in {project_id}");

                    let (status, _, _) = Runner::stream_output(
                        &mut runner.cmd_rustc(["/home/main.rs"]),
                        stream!(broadcast.clone(), OutputChannel::Stdout),
                        stream!(broadcast.clone(), OutputChannel::Stderr),
                    )
                    .await
                    .map_err(|err| {
                        log::error!("[Execute] compilation failed in {project_id}: {err}");
                        _ = broadcast.send(ServerMessage::SyncOutput {
                            channel: OutputChannel::Stderr,
                            buf: err.to_string().into_bytes(),
                        });
                        _ = broadcast.send(ServerMessage::SyncOutputEnd { exit_code: 126 });
                        ServerMessageError::None
                    })?;

                    if !status.success() {
                        log::error!("[Execute] compilation failed in {project_id}");
                        _ = broadcast.send(ServerMessage::SyncOutputEnd { exit_code: 126 });

                        return Err(ServerMessageError::None);
                    }

                    log::trace!("[Execute] patching in {project_id}");

                    let output = runner.patch_binary("/home/main").await.map_err(|err| {
                        log::error!("[Execute] patching failed in {project_id}: {err}");
                        _ = broadcast.send(ServerMessage::SyncOutput {
                            channel: OutputChannel::Stderr,
                            buf: err.to_string().into_bytes(),
                        });
                        _ = broadcast.send(ServerMessage::SyncOutputEnd { exit_code: 126 });
                        ServerMessageError::None
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
                        _ = broadcast.send(ServerMessage::SyncOutputEnd { exit_code: 126 });

                        return Err(ServerMessageError::None);
                    }
                    log::trace!("[Execute] running in {project_id}");

                    let (exit_code, _, _) = Runner::stream_output(
                        &mut runner.cmd("/home/main", [] as [&str; 0]),
                        stream!(broadcast.clone(), OutputChannel::Stdout),
                        stream!(broadcast.clone(), OutputChannel::Stderr),
                    )
                    .await
                    .map_err(|err| {
                        log::trace!("[Execute] run failed in {project_id}: {err}");
                        _ = broadcast.send(ServerMessage::SyncOutput {
                            channel: OutputChannel::Stderr,
                            buf: err.to_string().into_bytes(),
                        });
                        _ = broadcast.send(ServerMessage::SyncOutputEnd { exit_code: 126 });
                        ServerMessageError::None
                    })?;

                    log::trace!("[Execute] finish in {project_id}");

                    _ = broadcast.send(ServerMessage::SyncOutputEnd {
                        exit_code: exit_code.code as u8,
                    });

                    Ok(())
                });

                Err(ServerMessageError::None)
            }
            ClientMessage::FileCreate { file } => {
                self.access.need_editor()?;

                let project = manager.get_project_mut(self.project_id)?;

                project.add_file(file, Document::new()).await;

                let msg = ServerMessage::ProjectFiles {
                    files: project.get_files(),
                };

                _ = project.broadcast.send(msg);

                Err(ServerMessageError::None)
            }
            ClientMessage::FileDelete { file } => {
                self.access.need_editor()?;

                let project = manager.get_project_mut(self.project_id)?;

                if let Some(_) = project.rm_file(&file) {
                    let msg = ServerMessage::ProjectFiles {
                        files: project.get_files(),
                    };

                    _ = project.broadcast.send(msg);

                    Err(ServerMessageError::None)
                } else {
                    Err(ServerMessageError::FileNotFound(file))
                }
            }
            ClientMessage::PermitAccess { user_id, access } => {
                let project = manager.get_project_mut(self.project_id)?;

                if project.owner != self.user_info.id {
                    return Err(ServerMessageError::NotOwner);
                }

                log::info!("User {user_id} accepted");

                project.permit_access(user_id.clone(), access);

                _ = project
                    .broadcast
                    .send(ServerMessage::UpdateAccess { user_id, access });

                Err(ServerMessageError::None)
            }
            ClientMessage::Sync {
                file,
                revision,
                actions,
            } => {
                self.access.need_editor()?;

                let project = manager.get_project_mut(self.project_id)?;

                let doc = project.get_file_mut(&file).ok_or_else(|| {
                    log::error!("File {file:?} not found in {:?}", self.project_id);
                    ServerMessageError::FileNotFound(file.clone())
                })?;

                doc.compose(revision, actions);

                dbg!(&doc.buffer);

                let msg = ServerMessage::Sync {
                    file,
                    revision: doc.revision(),
                    actions: doc.history.clone(),
                };

                _ = project.broadcast.send(msg);

                Err(ServerMessageError::None)
            }

            ClientMessage::SyncCursor { file, cursors } => {
                self.access.need_editor()?;

                let project = manager.get_project_mut(self.project_id)?;

                let doc = project.get_file_mut(&file).ok_or_else(|| {
                    log::error!("File {file:?} not found in {:?}", self.project_id);
                    ServerMessageError::FileNotFound(file.clone())
                })?;

                if cursors.is_empty() {
                    doc.cursors.remove(&self.user_info.id);
                } else {
                    doc.cursors.insert(self.user_info.id.clone(), cursors);
                }

                let cursors = doc.cursors.clone();

                _ = project
                    .broadcast
                    .send(ServerMessage::SyncCursors { file, cursors });

                Err(ServerMessageError::None)
            }
            ClientMessage::SyncFiles => {
                self.access.need_read()?;

                let project = manager.get_project_mut(self.project_id)?;

                Ok(ServerMessage::ProjectFiles {
                    files: project.get_files().clone(),
                })
            }
        }
    }
}
