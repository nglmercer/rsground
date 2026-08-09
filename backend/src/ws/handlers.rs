use std::collections::HashMap;

use actix_ws as ws;
use futures::StreamExt as _;

use crate::collab::Document;
use crate::constants::{collaboration, websocket};
use crate::project::{AccessLevel, MAX_PROJECT_FILES, MAX_PROJECT_PASSWORD_BYTES};
use crate::utils::{ArcStr, ToStream};
use crate::ws::messages::{ClientMessage, ServerMessage, ServerMessageError};
use crate::ws::ws_ext::SessionExt;

use super::lsp::{validate_lsp_message, MAX_LSP_MESSAGE_BYTES};
use super::messages::InternalMessage;
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

    pub async fn handle_welcome(&mut self, ctx: &mut ws::Session) {
        // Don't send welcome when is in queue
        if !self.access.can_read() {
            return;
        }

        Self::handle_ws_response(ctx, self.compose_welcome().await).await
    }

    async fn compose_welcome(&mut self) -> Result<ServerMessage, ServerMessageError> {
        let project = self.app_state.get_project(self.project_id).await?;
        let project = project.read().await;

        _ = project.broadcast.send(ServerMessage::UserConnected {
            user_id: self.user_info.id.clone(),
            user_name: self.user_info.name.clone(),
        });

        self.sync_docs = (&project.documents)
            .to_stream()
            .map(async |(k, v)| (k.clone(), (v.clone(), v.revision().await)))
            .buffer_unordered(websocket::DOCUMENT_CONCURRENCY)
            .collect()
            .await;

        let files = project.get_files().await;

        let users = (&project.allowed_users)
            .to_stream()
            .filter_map(async |(user, access)| {
                Some((
                    user.clone(),
                    (self.app_state.get_username(user).await?, *access),
                ))
            })
            .collect::<HashMap<ArcStr, (ArcStr, AccessLevel)>>()
            .await;

        let requests = if project.owner == self.user_info.id {
            Some(
                (&project.requests)
                    .to_stream()
                    .filter_map(async |user| {
                        Some((user.clone(), self.app_state.get_username(user).await?))
                    })
                    .collect::<HashMap<ArcStr, ArcStr>>()
                    .await,
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

    pub async fn handle_internal(&mut self, msg: InternalMessage, ctx: &mut ws::Session) {
        // Don't send internal updates when is in queue
        if !self.access.can_read() {
            return;
        }

        Self::handle_ws_response(ctx, self.compose_internal(msg).await).await
    }

    async fn compose_internal(
        &mut self,
        msg: InternalMessage,
    ) -> Result<ServerMessage, ServerMessageError> {
        match msg {
            InternalMessage::Edit { path } => {
                // A client can submit an edit immediately after receiving the
                // project-files broadcast. The file-create notification uses
                // a separate channel, so recover the document here instead
                // of dropping the first edit if that notification has not
                // been scheduled yet.
                if !self.sync_docs.contains_key(&path) {
                    let project = self.app_state.get_project(self.project_id).await?;
                    let project = project.read().await;
                    let Some(doc) = project.get_file(&path) else {
                        return Err(ServerMessageError::FileNotFound(path));
                    };
                    self.sync_docs
                        .insert(path.clone(), (doc, collaboration::INITIAL_REVISION));
                }

                let Some((doc, revision)) = self.sync_docs.get_mut(&path) else {
                    return Err(ServerMessageError::FileNotFound(path));
                };

                if doc.revision().await > *revision {
                    let (new_revision, actions) = doc.send_history(*revision).await;

                    let msg = if let Some(actions) = actions {
                        Ok(ServerMessage::Sync {
                            file: path,
                            revision: *revision,
                            actions,
                        })
                    } else {
                        Err(ServerMessageError::None)
                    };

                    *revision = new_revision;

                    msg
                } else {
                    Err(ServerMessageError::None)
                }
            }
            InternalMessage::Create { path, doc } => {
                self.sync_docs
                    .insert(path, (doc, collaboration::INITIAL_REVISION));

                Err(ServerMessageError::None)
            }
            InternalMessage::Delete { path } => {
                self.sync_docs.remove(&path);

                Err(ServerMessageError::None)
            }
        }
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
            ServerMessage::RequestAccess { .. } => {
                if let Ok(p) = self.app_state.get_project(self.project_id).await {
                    if p.read().await.owner == self.user_info.id {
                        _ = ctx.text_json(&msg).await
                    }
                }
            }

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
                if text == websocket::PING {
                    return;
                }

                log::trace!("Received websocket client message");

                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        let msg = self.handle_client_message(ctx, client_msg).await;

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

        match msg {
            ClientMessage::Config {
                name,
                is_public,
                password,
            } => {
                let project = self.app_state.get_project(self.project_id).await?;
                let mut project = project.write().await;

                if project.owner != self.user_info.id {
                    return Err(ServerMessageError::NotOwner);
                }

                if let Some(name) = name {
                    project.name = name;
                }

                if let Some(is_public) = is_public {
                    project.is_public = is_public;
                }

                if let Some(password) = password {
                    if password.is_empty() {
                        project.set_password(None).map_err(|_| {
                            ServerMessageError::InvalidOperation(
                                "cannot clear project password".to_owned(),
                            )
                        })?;
                    } else {
                        if password.len() > MAX_PROJECT_PASSWORD_BYTES {
                            return Err(ServerMessageError::PasswordTooLong);
                        }

                        if project.is_public {
                            project.set_password(Some(&password)).map_err(|_| {
                                ServerMessageError::InvalidOperation(
                                    "cannot set project password".to_owned(),
                                )
                            })?;
                        }
                    }
                }

                _ = project.broadcast.send(ServerMessage::ProjectConfig {
                    name: project.name.clone(),
                    is_public: project.is_public,
                });

                Err(ServerMessageError::None)
            }
            ClientMessage::Execute => {
                self.access.need_editor()?;

                let project = self.app_state.get_project(self.project_id).await?;
                let project = project.read().await;

                project.execute().await;

                Err(ServerMessageError::None)
            }
            ClientMessage::FileCreate { file } => {
                self.access.need_editor()?;

                if !crate::project::Project::is_valid_file_path(&file) {
                    return Err(ServerMessageError::InvalidFilePath(file));
                }

                let project = self.app_state.get_project(self.project_id).await?;
                let mut project = project.write().await;

                if project.documents.len() >= MAX_PROJECT_FILES {
                    return Err(ServerMessageError::ProjectFileLimit);
                }

                if project.documents.contains_key(&file) {
                    return Err(ServerMessageError::FileAlreadyExists(file));
                }

                let new_doc = project.add_file(file.clone(), Document::new()).await;

                _ = project.internal.send(InternalMessage::Create {
                    path: file,
                    doc: new_doc,
                });

                let msg = ServerMessage::ProjectFiles {
                    files: project.get_files().await,
                };

                _ = project.broadcast.send(msg);

                Err(ServerMessageError::None)
            }
            ClientMessage::FileDelete { file } => {
                self.access.need_editor()?;

                let project = self.app_state.get_project(self.project_id).await?;
                let mut project = project.write().await;

                if project.rm_file(&file).is_some() {
                    _ = project
                        .get_runner()
                        .remove_file(&file)
                        .await
                        .inspect_err(|err| {
                            log::error!("Cannot remove runner file {file:?}: {err}")
                        });

                    let msg = ServerMessage::ProjectFiles {
                        files: project.get_files().await,
                    };

                    _ = project
                        .internal
                        .send(InternalMessage::Delete { path: file });

                    _ = project.broadcast.send(msg);

                    Err(ServerMessageError::None)
                } else {
                    Err(ServerMessageError::FileNotFound(file))
                }
            }
            ClientMessage::PermitAccess { user_id, access } => {
                let project = self.app_state.get_project(self.project_id).await?;
                let mut project = project.write().await;

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
            ClientMessage::StopExecute => {
                self.access.need_editor()?;

                let project = self.app_state.get_project(self.project_id).await?;
                let project = project.read().await;

                log::trace!("Stop process in {}", self.project_id);
                project.stop_execute();

                Err(ServerMessageError::None)
            }
            ClientMessage::Sync {
                file,
                revision,
                actions,
            } => {
                self.access.need_editor()?;

                let project = self.app_state.get_project(self.project_id).await?;
                let project = project.read().await;
                let runner = project.get_runner();

                let doc = project.get_file(&file).ok_or_else(|| {
                    log::error!("File {file:?} not found in {:?}", self.project_id);
                    ServerMessageError::FileNotFound(file.clone())
                })?;

                if let Err(err) = doc
                    .compose(self.session_id.clone(), revision, actions)
                    .await
                {
                    return Err(ServerMessageError::InvalidOperation(err));
                }

                _ = runner
                    .create_file(&file, &doc.text().await)
                    .await
                    .inspect_err(|err| log::error!("{err}"));

                _ = project.internal.send(InternalMessage::Edit { path: file });

                Err(ServerMessageError::None)
            }

            ClientMessage::SyncCursor { file, cursors } => {
                self.access.need_editor()?;

                let project = self.app_state.get_project(self.project_id).await?;
                let project = project.read().await;

                let doc = project.get_file(&file).ok_or_else(|| {
                    log::error!("File {file:?} not found in {:?}", self.project_id);
                    ServerMessageError::FileNotFound(file.clone())
                })?;

                let mut doc = doc.state_mut().await;

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

                let project = self.app_state.get_project(self.project_id).await?;
                let project = project.read().await;

                Ok(ServerMessage::ProjectFiles {
                    files: project.get_files().await,
                })
            }
            ClientMessage::Lsp { message } => {
                validate_lsp_message(&message).map_err(|error| {
                    ServerMessageError::InvalidOperation(format!(
                        "invalid language-server message: {error}"
                    ))
                })?;

                let message = serde_json::to_string(&message).map_err(|error| {
                    ServerMessageError::InvalidOperation(format!(
                        "cannot serialize language-server message: {error}"
                    ))
                })?;

                if message.len() > MAX_LSP_MESSAGE_BYTES {
                    return Err(ServerMessageError::InvalidOperation(
                        "language-server message exceeds the maximum size".to_owned(),
                    ));
                }

                self.send_lsp(message).await.map_err(|error| {
                    ServerMessageError::InvalidOperation(format!(
                        "language server unavailable: {error}"
                    ))
                })?;

                Err(ServerMessageError::None)
            }
        }
    }
}
