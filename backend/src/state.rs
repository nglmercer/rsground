use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::project::ProjectManager;

#[derive(Clone)]
pub struct AppState {
    pub manager: Arc<Mutex<ProjectManager>>,
    pub usernames: Arc<Mutex<HashMap<String, String>>>,
}

impl AppState {
    /// Lock project manager, and recover from poison mutex
    pub fn get_manager(&self) -> MutexGuard<'_, ProjectManager> {
        self.manager.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Get username of user with provided id
    pub fn get_username(&self, id: &String) -> Option<String> {
        self.usernames
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .cloned()
    }

    /// Insert username of user with provided id
    pub fn add_username(&self, id: String, username: String) {
        log::trace!("New username registered: {id:?} = {username:?}");
        self.usernames
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, username);
    }
}
