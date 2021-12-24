use super::canned_response::ApplicationError;
use crate::backend::{BackendError};
use tracing::info;
use warp::{Filter, Rejection, Reply};
use super::prelude::*;

use serde::{Deserialize, Serialize};

type GetOrganization = CreateOrganization;

#[derive(Debug, Serialize, Deserialize)]
struct CreateOrganization {
    pub org: String,
}

impl From<crate::backend::models::DataStoreOrganization> for GetOrganization {
    fn from(model: crate::backend::models::DataStoreOrganization) -> Self {
        (&model).into()
    }
}

impl From<&crate::backend::models::DataStoreOrganization> for GetOrganization {
    fn from(model: &crate::backend::models::DataStoreOrganization) -> Self {
        Self {
            org: model.name.clone(),
        }
    }
}

pub fn create_org_api(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    create_org(db.clone())
    .or(delete_org(db.clone()))
    .or(get_orgs(db.clone()))
    .or(get_an_org(db.clone()))
}

fn create_org(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("POST /api/org");
    warp::path!("api" / "org")
        .and(warp::post())
        .and(json_body::<CreateOrganization>())
        .and(with_db(db))
        .and_then(create_org_impl)
}

async fn create_org_impl(
    org: CreateOrganization,
    db: crate::Db,
) -> Result<impl Reply, Rejection> {
    let result = db.create_organization(&org.org).await;
    let result = result.map(|org| GetOrganization { org: org.name });
    wrap_body(result.map_err(ApplicationError::from_context))
}

fn delete_org(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("DELETE /api/org/{{org}}");
    warp::path!("api" / "org" / String)
        .and(warp::delete())
        .and(with_db(db))
        .and_then(delete_org_impl)
}

async fn delete_org_impl(
    org_name: String,
    db: crate::Db,
) -> Result<impl Reply, Rejection> {
    let result = db.delete_organization(&org_name).await;
    let result: Result<DeleteStatus, BackendError> = result.map(DeleteStatus::from);
    wrap_body(result.map_err(ApplicationError::from_context))
}

fn get_orgs(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("GET /api/org");
    warp::path!("api" / "org")
        .and(warp::get())
        .and(warp::query::<ApiPagination>())
        .and(with_db(db))
        .and_then(get_orgs_impl)
}

async fn get_orgs_impl(
    pageination: ApiPagination,
    db: crate::Db,
) -> Result<impl Reply, Rejection> {
    let result = db.get_organizations(pageination.into()).await;
    let result: Result<DataWrapper<Vec<GetOrganization>>, BackendError> =
        result.map(|orgs_list| DataWrapper::new(orgs_list.orgs.iter().map(GetOrganization::from).collect()));
    wrap_body(result.map_err(ApplicationError::from_context))
}

fn get_an_org(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("GET /api/org/{{org}}");
    warp::path!("api" / "org" / String)
        .and(warp::get())
        .and(with_db(db))
        .and_then(get_an_org_impl)
}

async fn get_an_org_impl(
    org_name: String,
    db: crate::Db,
) -> Result<impl Reply, Rejection> {
    let result = db.get_organization(&org_name).await;
    let result: Result<GetOrganization, BackendError> = result.map(|org| GetOrganization::from(org));
    wrap_body(result.map_err(ApplicationError::from_context))
}