use std::collections::HashMap;
use std::sync::Arc;

use operational_transform::OperationSeq;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::collab::{Document, DocumentInfo, UserOperation};
use crate::project::AccessLevel;
use crate::utils::ArcStr;

#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ClientMessage {
    Config {
        name: Option<ArcStr>,
        is_public: Option<bool>,
        password: Option<String>,
    },
    Execute,
    FileCreate {
        file: ArcStr,
    },
    FileDelete {
        file: ArcStr,
    },
    PermitAccess {
        user_id: ArcStr,
        access: AccessLevel,
    },
    StopExecute,
    Sync {
        file: ArcStr,
        revision: usize,
        actions: OperationSeq,
    },
    SyncCursor {
        file: ArcStr,
        cursors: Vec<(u32, u32)>,
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
        name: ArcStr,
        is_public: bool,
    },
    ProjectFiles {
        /// List of all file paths
        files: HashMap<ArcStr, DocumentInfo>,
    },
    UpdateAccess {
        access: AccessLevel,
        user_id: ArcStr,
    },
    UserConnected {
        user_id: ArcStr,
        user_name: ArcStr,
    },
    RequestAccess {
        user_id: ArcStr,
        user_name: ArcStr,
    },
    Sync {
        file: ArcStr,
        revision: usize,
        actions: Vec<UserOperation>,
    },
    SyncCursors {
        file: ArcStr,
        cursors: HashMap<ArcStr, Vec<(u32, u32)>>,
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
        session_id: ArcStr,
        files: HashMap<ArcStr, DocumentInfo>,
        users: HashMap<ArcStr, (ArcStr, AccessLevel)>,
        // Only for owner
        requests: Option<HashMap<ArcStr, ArcStr>>,
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
    FileNotFound(ArcStr),

    #[error("File {0:?} already exists")]
    FileAlreadyExists(ArcStr),

    #[error("Invalid file path {0:?}")]
    InvalidFilePath(ArcStr),

    #[error("Invalid document operation: {0}")]
    InvalidOperation(String),

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

#[derive(Clone)]
pub enum InternalMessage {
    Edit { path: ArcStr },
    Create { path: ArcStr, doc: Arc<Document> },
    Delete { path: ArcStr },
}
