use std::collections::{HashMap, HashSet};
use std::path::{Component, Path};
use std::sync::Arc;

use actix::Addr;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use futures::StreamExt;
use rsground_runner::{error::RunnerError, Runner};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::auth::jwt::RgUserData;
use crate::collab::{Document, DocumentInfo};
use crate::constants::{filesystem, limits, project as project_constants, websocket};
use crate::http_errors::HttpErrors;
use crate::utils::{ArcStr, AsyncInto, ToStream};
use crate::ws::messages::{InternalMessage, ServerMessage};

use super::project_runner::{AbortNotify, Execute, ProjectExecuter};
use super::AccessLevel;

pub const MAX_PROJECT_FILES: usize = limits::MAX_PROJECT_FILES;
pub const MAX_PROJECT_NAME_CHARS: usize = limits::MAX_PROJECT_NAME_CHARS;
pub const MAX_PROJECT_PASSWORD_BYTES: usize = limits::MAX_PROJECT_PASSWORD_BYTES;
const MAX_FILE_PATH_BYTES: usize = limits::MAX_FILE_PATH_BYTES;

pub struct Project {
    pub id: Uuid,
    pub name: ArcStr,
    pub owner: ArcStr,
    pub documents: HashMap<ArcStr, Arc<Document>>,
    pub allowed_users: HashMap<ArcStr, AccessLevel>,
    pub requests: HashSet<ArcStr>,
    pub is_public: bool,
    /// Argon2 password hash. The plaintext password is never retained or
    /// returned by the API.
    pub password: Option<String>,
    pub internal: broadcast::Sender<InternalMessage>,
    pub broadcast: broadcast::Sender<ServerMessage>,
    runner: Arc<Runner>,
    executer: Addr<ProjectExecuter>,
    execution: AbortNotify,
}

impl Project {
    pub async fn new(owner: ArcStr, name: ArcStr) -> Result<Self, RunnerError> {
        let id = Uuid::new_v4();
        let broadcast = broadcast::channel(websocket::BROADCAST_CAPACITY).0;

        let (runner, execution, executer) = ProjectExecuter::start(id, broadcast.clone()).await?;

        Ok(Self {
            id,
            name,
            owner,
            documents: HashMap::new(),
            allowed_users: HashMap::new(),
            requests: HashSet::new(),
            is_public: true,
            password: None,
            internal: broadcast::channel(websocket::BROADCAST_CAPACITY).0,
            broadcast,
            runner,
            executer,
            execution,
        })
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
            && path.as_os_str().as_encoded_bytes().len() <= MAX_FILE_PATH_BYTES
            && !path.is_absolute()
            && !path
                .as_os_str()
                .as_encoded_bytes()
                .contains(&filesystem::NUL_BYTE)
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
    }

    pub fn is_valid_name(name: &str) -> bool {
        let trimmed = name.trim();
        !trimmed.is_empty()
            && trimmed.chars().count() <= MAX_PROJECT_NAME_CHARS
            && !trimmed.chars().any(char::is_control)
    }

    pub fn set_password(
        &mut self,
        password: Option<&str>,
    ) -> Result<(), argon2::password_hash::Error> {
        self.password = password
            .map(|password| {
                let salt = SaltString::generate(&mut OsRng);
                Argon2::default()
                    .hash_password(password.as_bytes(), &salt)
                    .map(|hash| hash.to_string())
            })
            .transpose()?;

        Ok(())
    }

    fn password_matches(&self, password: Option<&str>) -> bool {
        let (Some(hash), Some(password)) = (self.password.as_deref(), password) else {
            return false;
        };

        if password.len() > MAX_PROJECT_PASSWORD_BYTES {
            return false;
        }

        let Ok(hash) = PasswordHash::new(hash) else {
            return false;
        };

        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    }

    /// Get all file paths
    pub async fn get_files(&self) -> HashMap<ArcStr, DocumentInfo> {
        (&self.documents)
            .to_stream()
            .map(async |(path, doc)| (path.clone(), doc.async_into().await))
            .buffer_unordered(websocket::FILE_READ_CONCURRENCY)
            .collect()
            .await
    }

    pub async fn fork(&self, owner: ArcStr) -> Result<Project, RunnerError> {
        let name = if self.name.ends_with(project_constants::FORK_SUFFIX) {
            self.name.clone()
        } else {
            format!(
                "{}{suffix}",
                self.name,
                suffix = project_constants::FORK_SUFFIX
            )
            .as_str()
            .into()
        };

        let documents = (&self.documents)
            .to_stream()
            .map(async |(path, doc)| (path.clone(), doc.fork().await.into()))
            .buffer_unordered(websocket::DOCUMENT_CONCURRENCY)
            .collect::<HashMap<ArcStr, Arc<Document>>>()
            .await;

        let mut forked = Project::new(owner, name).await?;
        forked.documents = documents;
        forked.is_public = self.is_public;

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

        Ok(forked)
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
        } else if self.password.is_some() {
            if !self.password_matches(password.as_deref()) {
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
