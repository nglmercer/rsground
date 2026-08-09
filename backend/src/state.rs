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

#[cfg(test)]
mod tests {
    use super::AppState;
    use crate::project::ProjectManager;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    #[tokio::test]
    async fn enforces_user_capacity_without_rejecting_existing_users() {
        let state = AppState {
            manager: Arc::new(Mutex::new(ProjectManager::new())),
            usernames: Arc::new(Mutex::new(HashMap::new())),
            max_users: 1,
        };

        assert!(state.add_username("first".into(), "Ada".into()).await);
        assert!(
            state
                .add_username("first".into(), "Ada Lovelace".into())
                .await
        );
        assert_eq!(
            state.get_username("first").await.as_deref(),
            Some("Ada Lovelace")
        );
        assert!(!state.add_username("second".into(), "Grace".into()).await);
    }

    #[tokio::test]
    async fn reports_missing_projects() {
        let state = AppState {
            manager: Arc::new(Mutex::new(ProjectManager::new())),
            usernames: Arc::new(Mutex::new(HashMap::new())),
            max_users: 1,
        };

        let result = state.get_project(Uuid::new_v4()).await;
        assert!(matches!(
            result,
            Err(crate::ws::messages::ServerMessageError::ProjectNotFound(_))
        ));
    }
}
