use std::collections::{HashMap, HashSet};
use std::path::{Component, Path};
use std::sync::Arc;

use actix::Addr;
use futures::StreamExt;
use rsground_runner::Runner;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::auth::jwt::RgUserData;
use crate::collab::{Document, DocumentInfo};
use crate::http_errors::HttpErrors;
use crate::utils::{ArcStr, AsyncDefault, AsyncInto, ToStream, EMPTY_STR};
use crate::ws::messages::{InternalMessage, ServerMessage};

use super::project_runner::{AbortNotify, Execute, ProjectExecuter};
use super::AccessLevel;

pub struct Project {
    pub id: Uuid,
    pub name: ArcStr,
    pub owner: ArcStr,
    pub documents: HashMap<ArcStr, Arc<Document>>,
    pub allowed_users: HashMap<ArcStr, AccessLevel>,
    pub requests: HashSet<ArcStr>,
    pub is_public: bool,
    pub password: Option<String>,
    pub internal: broadcast::Sender<InternalMessage>,
    pub broadcast: broadcast::Sender<ServerMessage>,
    runner: Arc<Runner>,
    executer: Addr<ProjectExecuter>,
    execution: AbortNotify,
}

impl AsyncDefault for Project {
    async fn default() -> Self {
        let id = Uuid::new_v4();
        let broadcast = broadcast::channel(u8::MAX as usize).0;

        let (runner, execution, executer) = ProjectExecuter::start(id, broadcast.clone()).await;

        Self {
            id,
            name: EMPTY_STR.clone(),
            owner: EMPTY_STR.clone(),
            documents: HashMap::new(),
            allowed_users: HashMap::new(),
            requests: HashSet::new(),
            is_public: true,
            password: None,
            internal: broadcast::channel(u8::MAX as usize).0,
            broadcast,
            runner,
            executer,
            execution,
        }
    }
}

impl Project {
    pub async fn new(owner: ArcStr, name: ArcStr) -> Self {
        Project {
            name,
            owner,
            ..Project::default().await
        }
    }

    pub fn get_runner(&self) -> Arc<Runner> {
        self.runner.clone()
    }

    pub async fn execute(&self) {
        if self.execution.lock().is_ok_and(|e| e.is_some()) {
            return;
        }

        self.executer.do_send(Execute);
    }

    pub fn stop_execute(&self) {
        if let Some(execution) = self.execution.lock().unwrap().take() {
            _ = execution.send(());
        }
    }

    pub fn add_request(&mut self, user_info: &RgUserData) {
        if self.requests.insert(user_info.id.clone()) {
            _ = self.broadcast.send(ServerMessage::RequestAccess {
                user_id: user_info.id.clone(),
                user_name: user_info.name.clone(),
            });
        }
    }

    pub fn permit_access(&mut self, user_id: ArcStr, access: AccessLevel) {
        self.requests.remove(&user_id);
        self.allowed_users.insert(user_id, access);
    }

    pub fn get_file(&self, file_name: &str) -> Option<Arc<Document>> {
        self.documents.get(file_name).cloned()
    }

    pub async fn add_file(&mut self, path: impl Into<ArcStr>, document: Document) -> Arc<Document> {
        let path: ArcStr = path.into();

        _ = self
            .get_runner()
            .create_file(&path.to_string(), &document.text().await)
            .await
            .inspect_err(|err| log::error!("{err}"));

        self.documents
            .entry(path)
            .insert_entry(document.into())
            .get()
            .clone()
    }

    pub fn rm_file(&mut self, path: impl AsRef<str>) -> Option<Arc<Document>> {
        self.documents.remove(path.as_ref())
    }

    pub fn is_valid_file_path(path: &str) -> bool {
        let path = Path::new(path);

        !path.as_os_str().is_empty()
            && path.as_os_str().as_encoded_bytes().len() <= 256 * 1024
            && !path.is_absolute()
            && !path.as_os_str().as_encoded_bytes().contains(&0)
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
    }

    /// Get all file paths
    pub async fn get_files(&self) -> HashMap<ArcStr, DocumentInfo> {
        (&self.documents)
            .to_stream()
            .map(async |(path, doc)| (path.clone(), doc.async_into().await))
            .buffer_unordered(5)
            .collect()
            .await
    }

    pub async fn fork(&self, owner: ArcStr) -> Project {
        let name = if self.name.ends_with(" (fork)") {
            self.name.clone()
        } else {
            format!("{} (fork)", self.name).as_str().into()
        };

        let documents = (&self.documents)
            .to_stream()
            .map(async |(path, doc)| (path.clone(), doc.fork().await.into()))
            .buffer_unordered(5)
            .collect::<HashMap<ArcStr, Arc<Document>>>()
            .await;

        let forked = Project {
            name,
            owner,
            documents,
            is_public: self.is_public,
            ..Project::default().await
        };

        // The document map is copied in memory, but each project owns a
        // separate runner home. Populate that home as well or a fork would
        // open correctly and then fail on its first execution.
        let runner = forked.get_runner();
        for (path, document) in &forked.documents {
            _ = runner
                .create_file(&path.to_string(), &document.text().await)
                .await
                .inspect_err(|err| log::error!("Cannot seed fork file {path:?}: {err}"));
        }

        forked
    }

    pub fn join_project(
        &mut self,
        user_id: ArcStr,
        password: Option<String>,
    ) -> Result<AccessLevel, HttpErrors> {
        if let Some(access) = self.allowed_users.get(&user_id) {
            Ok(*access)
        } else if !self.is_public {
            Ok(AccessLevel::Queue)
        } else if let Some(ref p_password) = self.password {
            if password.is_none_or(|pass| &pass != p_password) {
                return Err(HttpErrors::InvalidPassword);
            }

            self.permit_access(user_id, AccessLevel::ReadOnly);
            Ok(AccessLevel::ReadOnly)
        } else {
            self.permit_access(user_id, AccessLevel::ReadOnly);
            Ok(AccessLevel::ReadOnly)
        }
    }
}
