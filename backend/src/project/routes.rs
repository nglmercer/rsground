use std::collections::HashMap;

use actix_error_proc::{proof_route, HttpResult};
use actix_web::{web, HttpRequest, HttpResponse};
use futures::StreamExt as _;
use serde_json::json;
use uuid::Uuid;

use crate::auth::jwt;
use crate::http_errors::HttpErrors;
use crate::project::{AccessLevel, Project, ProjectManagerError};
use crate::state::AppState;
use crate::utils::{ArcStr, ToStream};

#[proof_route(get("/project/{project_id}"))]
pub async fn get_project(
    app_state: web::Data<AppState>,
    project_id: web::Path<Uuid>,
    req: HttpRequest,
) -> HttpResult<HttpErrors> {
    let app_state = app_state.into_inner();
    let project_id = project_id.into_inner();
    let password = req
        .headers()
        .get("X-Project-Password")
        .and_then(|password| password.to_str().ok())
        .filter(|password| !password.is_empty())
        .map(str::to_owned);

    let user_info = jwt::get_user_info(&req)?;

    let Ok(project) = app_state.get_project(project_id).await else {
        return Err(HttpErrors::ProjectDoesNotExist);
    };
    let mut project = project.write().await;

    let access = project.join_project(user_info.id.clone(), password)?;

    if !access.can_read() {
        project.add_request(&user_info);

        return Ok(HttpResponse::Unauthorized().json(json!({
            "access": access,
            "id": project.id,
            "name": project.name,
            "is_public": project.is_public,
        })));
    }

    let users = (&project.allowed_users)
        .to_stream()
        .filter_map(async |(user, access)| {
            app_state
                .get_username(user)
                .await
                .map(|username| (user.clone(), (username, *access)))
        })
        .collect::<HashMap<ArcStr, (ArcStr, AccessLevel)>>()
        .await;

    let is_owner = project.owner == user_info.id;

    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(json!({
            "access": access,
            "id": project.id,
            "name": project.name,
            "owner": project.owner,
            "users": users,
            "is_public": project.is_public,
            "has_password": project.password.is_some(),
            "is_owner": is_owner
        })))
}

#[proof_route(post("/create/{name}"))]
pub async fn create_project(
    app_state: web::Data<AppState>,
    name: web::Path<ArcStr>,
    req: HttpRequest,
) -> HttpResult<HttpErrors> {
    let app_state = app_state.into_inner();
    let name = name.into_inner();

    let user_info = jwt::get_user_info(&req)?;

    if !Project::is_valid_name(&name) {
        return Err(HttpErrors::InvalidProjectName);
    }

    let mut manager = app_state.get_manager().await;
    let project = manager
        .new_project(&user_info, name)
        .await
        .map_err(map_manager_error)?;
    let mut project = project.write().await;

    project.permit_access(user_info.id.clone(), AccessLevel::Editor);

    Ok(HttpResponse::Created().json(json!({
        "id": project.id
    })))
}

#[proof_route(post("/fork/{project_id}"))]
pub async fn fork_project(
    app_state: web::Data<AppState>,
    project_id: web::Path<Uuid>,
    req: HttpRequest,
) -> HttpResult<HttpErrors> {
    let app_state = app_state.into_inner();
    let project_id = project_id.into_inner();

    let user_info = jwt::get_user_info(&req)?;

    let forked_project = {
        let Ok(project) = app_state.get_project(project_id).await else {
            return Err(HttpErrors::ProjectDoesNotExist);
        };
        let project = project.read().await;

        if !project
            .allowed_users
            .get(&user_info.id)
            .is_some_and(AccessLevel::can_read)
        {
            return Err(HttpErrors::NotAccessible);
        }

        project.fork(user_info.id.clone()).await.map_err(|error| {
            log::error!("Cannot create fork sandbox: {error}");
            HttpErrors::RunnerUnavailable
        })?
    };

    let project = app_state
        .get_manager()
        .await
        .add_project(forked_project)
        .map_err(map_manager_error)?;
    let mut project = project.write().await;

    project.permit_access(user_info.id.clone(), AccessLevel::Editor);

    Ok(HttpResponse::Created().json(json!({
        "id": project.id
    })))
}

fn map_manager_error(error: ProjectManagerError) -> HttpErrors {
    match error {
        ProjectManagerError::LimitReached => HttpErrors::ProjectLimitReached,
        ProjectManagerError::Runner(error) => {
            log::error!("Cannot create project sandbox: {error}");
            HttpErrors::RunnerUnavailable
        }
    }
}
