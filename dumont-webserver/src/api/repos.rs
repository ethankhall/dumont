use super::canned_response::ApplicationError;
use super::prelude::*;
use crate::backend::BackendError;
use std::collections::BTreeMap;
use tracing::info;
use warp::{Filter, Rejection, Reply};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRepository {
    pub repo: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetRepository {
    pub org: String,
    pub repo: String,
    pub labels: BTreeMap<String, String>,
}

impl From<&crate::backend::models::DataStoreRepository> for GetRepository {
    fn from(model: &crate::backend::models::DataStoreRepository) -> Self {
        Self {
            org: model.org_name.clone(),
            repo: model.repo_name.clone(),
            labels: model.labels.clone(),
        }
    }
}

impl From<crate::backend::models::DataStoreRepository> for GetRepository {
    fn from(model: crate::backend::models::DataStoreRepository) -> Self {
        Self {
            org: model.org_name.clone(),
            repo: model.repo_name.clone(),
            labels: model.labels.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRepository {
    pub labels: BTreeMap<String, String>,
}

pub fn create_repo_api(
    db: crate::Db,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    create_repo(db.clone())
        .or(get_repos(db.clone()))
        .or(get_repo(db.clone()))
        .or(delete_repo(db.clone()))
        .or(update_repo(db.clone()))
}

fn create_repo(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("POST /api/org/{{org}}/repo");
    warp::path!("api" / "org" / String / "repo")
        .and(warp::post())
        .and(json_body::<CreateRepository>())
        .and(with_db(db))
        .and_then(create_repo_impl)
}

async fn create_repo_impl(
    org: String,
    repo: CreateRepository,
    db: crate::Db,
) -> Result<impl Reply, Rejection> {
    let result = db.create_repo(&org, &repo.repo, repo.labels).await;
    let result = result.map(GetRepository::from);
    wrap_body(result.map_err(ApplicationError::from_context))
}

fn get_repos(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("GET /api/org/{{org}}/repo");
    warp::path!("api" / "org" / String / "repo")
        .and(warp::get())
        .and(warp::query::<ApiPagination>())
        .and(with_db(db))
        .and_then(get_repos_impl)
}

async fn get_repos_impl(
    org: String,
    pageination: ApiPagination,
    db: crate::Db,
) -> Result<impl Reply, Rejection> {
    let result = db.get_repos(&org, pageination.into()).await;
    let result: Result<DataWrapper<Vec<GetRepository>>, BackendError> = result.map(|repo_list| {
        DataWrapper::new(repo_list.repos.iter().map(GetRepository::from).collect())
    });
    wrap_body(result.map_err(ApplicationError::from_context))
}

fn get_repo(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("GET /api/org/{{org}}/repo/{{repo}}");
    warp::path!("api" / "org" / String / "repo" / String)
        .and(warp::get())
        .and(with_db(db))
        .and_then(get_repo_impl)
}

async fn get_repo_impl(org: String, repo: String, db: crate::Db) -> Result<impl Reply, Rejection> {
    let result = db.get_repo(&org, &repo).await;
    let result: Result<GetRepository, BackendError> = result.map(GetRepository::from);
    wrap_body(result.map_err(ApplicationError::from_context))
}

fn delete_repo(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("DELETE /api/org/{{org}}/repo/{{repo}}");
    warp::path!("api" / "org" / String / "repo" / String)
        .and(warp::delete())
        .and(with_db(db))
        .and_then(delete_repo_impl)
}

async fn delete_repo_impl(
    org: String,
    repo: String,
    db: crate::Db,
) -> Result<impl Reply, Rejection> {
    let result = db.delete_repo(&org, &repo).await;
    let result: Result<DeleteStatus, BackendError> = result.map(DeleteStatus::from);
    wrap_body(result.map_err(ApplicationError::from_context))
}

fn update_repo(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("PUT /api/org/{{org}}/repo/{{repo}}");
    warp::path!("api" / "org" / String / "repo" / String)
        .and(warp::put())
        .and(json_body::<UpdateRepository>())
        .and(with_db(db))
        .and_then(update_repo_impl)
}

async fn update_repo_impl(
    org: String,
    repo: String,
    update: UpdateRepository,
    db: crate::Db,
) -> Result<impl Reply, Rejection> {
    if let Err(err) = db.update_repo(&org, &repo, update.labels).await {
        return wrap_body(Err(ApplicationError::from_context(err)));
    }

    let result = db.get_repo(&org, &repo).await;
    let result: Result<GetRepository, BackendError> = result.map(GetRepository::from);
    wrap_body(result.map_err(ApplicationError::from_context))
}
