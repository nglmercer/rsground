use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use actix_ws as ws;
use futures::StreamExt;
use rsground_runner::{Child, Runner};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::auth::jwt::RgUserData;
use crate::collab::Document;
use crate::constants::websocket;
use crate::http_errors::HttpErrors;
use crate::project::AccessLevel;
use crate::state::AppState;
use crate::utils::ArcStr;

use super::lsp::LspProcess;
use super::messages::{InternalMessage, ServerMessage};
use super::ws_ext::SessionExt;

pub struct RgWebsocket {
    pub app_state: AppState,
    pub access: AccessLevel,
    pub internal: broadcast::Receiver<InternalMessage>,
    pub broadcast: broadcast::Receiver<ServerMessage>,
    pub project_id: Uuid,
    pub session_id: ArcStr,
    pub user_info: RgUserData,
    pub sync_docs: HashMap<ArcStr, (Arc<Document>, usize)>,
    runner: Arc<Runner>,
    lsp_incoming_sender: mpsc::Sender<String>,
    lsp_incoming: Option<mpsc::Receiver<String>>,
    lsp_sender: Option<mpsc::Sender<String>>,
    lsp_child: Option<Child>,
}

impl RgWebsocket {
    pub async fn join_project(
        app_state: AppState,
        user_info: RgUserData,
        project_id: Uuid,
        password: Option<String>,
    ) -> Result<Self, HttpErrors> {
        let (internal, broadcast, access, runner) = {
            let Ok(project) = app_state.get_project(project_id).await else {
                return Err(HttpErrors::ProjectDoesNotExist);
            };
            let mut project = project.write().await;

            (
                project.internal.subscribe(),
                project.broadcast.subscribe(),
                project.join_project(user_info.id.clone(), password)?,
                project.get_runner(),
            )
        };

        let (lsp_incoming_sender, lsp_incoming) = mpsc::channel(64);

        let ws = Self {
            access,
            app_state,
            broadcast,
            internal,
            project_id,
            sync_docs: Default::default(),
            session_id: Uuid::new_v4().to_string().as_str().into(),
            user_info,
            runner,
            lsp_incoming_sender,
            lsp_incoming: Some(lsp_incoming),
            lsp_sender: None,
            lsp_child: None,
        };

        Ok(ws)
    }

    pub fn start(mut self, mut session: ws::Session, mut stream: ws::AggregatedMessageStream) {
        actix::spawn(async move {
            self.handle_welcome(&mut session).await;

            let mut lsp_incoming = self
                .lsp_incoming
                .take()
                .expect("LSP incoming channel must be initialized");

            let mut ping =
                tokio::time::interval(Duration::from_secs(websocket::HEARTBEAT_INTERVAL_SECS));
            // `interval` fires immediately on its first tick. Delay the
            // first heartbeat so it cannot overtake the welcome/connection
            // notifications during the handshake.
            ping.tick().await;

            loop {
                tokio::select! {
                    _ = ping.tick() => {
                        _ = session.text(websocket::PING).await;
                    },
                    Ok(msg) = self.internal.recv() => {
                        self.handle_internal(msg, &mut session).await;
                    }
                    Ok(msg) = self.broadcast.recv() => {
                        self.handle_broadcast(msg, &mut session).await;
                    },
                    Some(message) = lsp_incoming.recv() => {
                        match serde_json::from_str(&message) {
                            Ok(message) => {
                                _ = session.text_json(&ServerMessage::Lsp { message }).await;
                            }
                            Err(error) => {
                                log::debug!("Ignoring invalid Rust Analyzer message: {error}");
                            }
                        }
                    },
                    msg = stream.next() => {
                        let Some(msg) = msg else {
                            break;
                        };

                        self.handle_ws_msg(msg, &mut session).await;
                    }
                }
            }

            drop(lsp_incoming);
            self.stop_lsp().await;
        });
    }

    async fn start_lsp(&mut self) -> Result<(), String> {
        if self.lsp_sender.is_some() {
            return Ok(());
        }

        let process = LspProcess::start(&self.runner, self.lsp_incoming_sender.clone())
            .map_err(|error| error.to_string())?;

        self.lsp_sender = Some(process.outgoing);
        self.lsp_child = Some(process.child);
        Ok(())
    }

    pub async fn send_lsp(&mut self, message: String) -> Result<(), String> {
        self.start_lsp().await?;

        self.lsp_sender
            .as_ref()
            .expect("LSP sender must be initialized")
            .send(message)
            .await
            .map_err(|error| error.to_string())
    }

    async fn stop_lsp(&mut self) {
        self.lsp_sender.take();

        let Some(mut child) = self.lsp_child.take() else {
            return;
        };

        _ = tokio::task::spawn_blocking(move || {
            _ = child.kill();
            _ = child.wait();
        })
        .await;
    }
}
