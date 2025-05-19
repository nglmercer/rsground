use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::collab::{Action, DocumentInfo};
use crate::project::AccessLevel;

#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ClientMessage {
    Config {
        name: Option<String>,
        is_public: Option<bool>,
        password: Option<String>,
    },
    Execute,
    FileCreate {
        file: String,
    },
    FileDelete {
        file: String,
    },
    PermitAccess {
        user_id: String,
        access: AccessLevel,
    },
    Sync {
        file: String,
        revision: usize,
        actions: Vec<Action>,
    },
    SyncCursor {
        file: String,
        cursors: Vec<(usize, usize)>,
    },
    SyncFiles,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ServerMessage {
    Error {
        message: String,
    },
    ProjectConfig {
        name: String,
        is_public: bool,
        password: Option<String>,
    },
    ProjectFiles {
        /// List of all file paths
        files: HashMap<String, DocumentInfo>,
    },
    UpdateAccess {
        access: AccessLevel,
        user_id: String,
    },
    UserConnected {
        user_id: String,
        user_name: String,
    },
    RequestAccess {
        user_id: String,
        user_name: String,
    },
    Sync {
        file: String,
        revision: usize,
        actions: Vec<Action>,
    },
    SyncCursors {
        file: String,
        cursors: HashMap<String, Vec<(usize, usize)>>,
    },
    SyncOutput {
        channel: OutputChannel,
        buf: Vec<u8>,
    },
    SyncOutputStart,
    SyncOutputEnd {
        exit_code: u8,
    },
    Welcome {
        session_id: String,
        files: HashMap<String, DocumentInfo>,
        users: HashMap<String, (String, AccessLevel)>,
        // Only for owner
        requests: Option<HashMap<String, String>>,
    },
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputChannel {
    Stdout,
    Stderr,
}

#[derive(Debug, thiserror::Error)]
pub enum ServerMessageError {
    #[error("")]
    None,

    #[error("File {0:?} not found")]
    FileNotFound(String),

    #[error("You don't have read permission")]
    NotAccessible,

    #[error("You are not the owner: fork the project")]
    NotOwner,

    #[error("Project {0:?} not found")]
    ProjectNotFound(Uuid),

    #[error("Denied permission: read-only access")]
    ReadonlyPermission,
}

impl From<ServerMessageError> for ServerMessage {
    fn from(value: ServerMessageError) -> ServerMessage {
        ServerMessage::Error {
            message: value.to_string(),
        }
    }
}
