use std::collections::HashMap;
use std::sync::Arc;

use rsground_runner::error::RunnerError;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::auth::jwt::RgUserData;
use crate::collab::Document;
use crate::utils::ArcStr;
use crate::ws::messages::ServerMessageError;

use super::Project;

const MAIN_RS: &str = r#"fn main() {
    println!("Hello World");
}"#;
pub const DEFAULT_MAX_PROJECTS: usize = 64;

pub struct ProjectManager {
    projects: HashMap<Uuid, Arc<RwLock<Project>>>,
    max_projects: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectManagerError {
    #[error("maximum active project limit reached")]
    LimitReached,
    #[error("runner unavailable: {0}")]
    Runner(#[from] RunnerError),
}

impl ProjectManager {
    pub fn new() -> Self {
        ProjectManager {
            projects: HashMap::new(),
            max_projects: configured_max_projects(),
        }
    }

    pub async fn new_project(
        &mut self,
        owner: &RgUserData,
        name: ArcStr,
    ) -> Result<Arc<RwLock<Project>>, ProjectManagerError> {
        if !self.has_capacity() {
            return Err(ProjectManagerError::LimitReached);
        }

        let mut project = Project::new(owner.id.clone(), name).await?;

        project
            .add_file("main.rs", Document::new_with(MAIN_RS.to_string()))
            .await;

        self.add_project(project)
    }

    pub fn add_project(
        &mut self,
        project: Project,
    ) -> Result<Arc<RwLock<Project>>, ProjectManagerError> {
        if !self.has_capacity() {
            return Err(ProjectManagerError::LimitReached);
        }

        log::info!("New project {}: {}", project.id, project.name);
        Ok(self
            .projects
            .entry(project.id)
            .insert_entry(RwLock::new(project).into())
            .get()
            .clone())
    }

    pub fn has_capacity(&self) -> bool {
        self.projects.len() < self.max_projects
    }

    pub fn get_project(&self, id: Uuid) -> Result<Arc<RwLock<Project>>, ServerMessageError> {
        self.projects
            .get(&id)
            .cloned()
            .ok_or(ServerMessageError::ProjectNotFound(id))
    }
}

fn configured_max_projects() -> usize {
    std::env::var("RSGROUND_MAX_PROJECTS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|limit: &usize| *limit > 0)
        .unwrap_or(DEFAULT_MAX_PROJECTS)
}
