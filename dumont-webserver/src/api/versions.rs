use super::canned_response::ApplicationError;
use super::prelude::*;
use tracing::info;
use warp::{Filter, Rejection, Reply};

use serde::{Deserialize, Serialize};

type RevisionLabels = crate::models::GenericLabels;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateVersion {
    pub version: String,
    pub scm_id: String,
    #[serde(flatten)]
    pub labels: RevisionLabels,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetVersion {
    pub version: String,
    pub scm_id: String,
    #[serde(flatten)]
    pub labels: RevisionLabels,
}

impl From<crate::backend::models::DataStoreRevision> for GetVersion {
    fn from(source: crate::backend::models::DataStoreRevision) -> Self {
        Self {
            version: source.version,
            scm_id: source.scm_id,
            labels: source.labels.clone(),
        }
    }
}

pub fn create_version_api(
    db: crate::Db,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    create_version(db.clone())
}

fn create_version(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("POST /api/org/{{org}}/repo/{{repo}}/version");
    warp::path!("api" / "org" / String / "repo" / String / "version")
        .and(warp::post())
        .and(json_body::<CreateVersion>())
        .and(with_db(db))
        .and_then(create_version_impl)
}

async fn create_version_impl(
    org: String,
    repo: String,
    version: CreateVersion,
    db: crate::Db,
) -> Result<impl Reply, Rejection> {
    let result = db
        .create_version(
            &org,
            &repo,
            &version.version,
            &version.scm_id,
            version.labels.labels,
        )
        .await;
    let result = result.map(GetVersion::from);
    wrap_body(result.map_err(ApplicationError::from_context))
}
