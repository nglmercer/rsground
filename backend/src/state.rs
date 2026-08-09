use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, MutexGuard, RwLock};
use uuid::Uuid;

use crate::project::{Project, ProjectManager};
use crate::utils::ArcStr;
use crate::ws::messages::ServerMessageError;

#[derive(Clone)]
pub struct AppState {
    pub manager: Arc<Mutex<ProjectManager>>,
    pub usernames: Arc<Mutex<HashMap<ArcStr, ArcStr>>>,
    pub(crate) max_users: usize,
}

impl AppState {
    /// Lock project manager, and recover from poison mutex
    pub async fn get_manager(&self) -> MutexGuard<'_, ProjectManager> {
        self.manager.lock().await
    }

    pub async fn get_project(&self, id: Uuid) -> Result<Arc<RwLock<Project>>, ServerMessageError> {
        self.get_manager().await.get_project(id)
    }

    /// Get username of user with provided id
    pub async fn get_username(&self, id: &str) -> Option<ArcStr> {
        self.usernames.lock().await.get(id).cloned()
    }

    /// Insert username of user with provided id
    pub async fn add_username(&self, id: ArcStr, username: ArcStr) -> bool {
        log::trace!("New username registered: {id:?} = {username:?}");
        let mut usernames = self.usernames.lock().await;
        if !usernames.contains_key(&id) && usernames.len() >= self.max_users {
            return false;
        }

        usernames.insert(id, username);
        true
    }
}
