use super::prelude::*;
use crate::backend::BackendError;
use tracing::info;
use tracing_attributes::instrument;
use warp::{Filter, Rejection, Reply};

use serde::{Deserialize, Serialize};

type RevisionLabels = crate::models::GenericLabels;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateVersion {
    pub version: String,
    #[serde(default, flatten)]
    pub labels: RevisionLabels,
}

#[test]
fn validate_create_version_deserialize() {
    use json::object;

    let _foo: CreateVersion = serde_json::from_str(&json::stringify(object! {
        "version":  "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
        "labels": {
            "foo": "bar"
        }
    }))
    .unwrap();

    let _foo: CreateVersion = serde_json::from_str(&json::stringify(object! {
        "version":  "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
    }))
    .unwrap();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateVersion {
    #[serde(flatten)]
    pub labels: RevisionLabels,
}

#[test]
fn validate_update_version_deserialize() {
    use json::object;

    let _foo: UpdateVersion = serde_json::from_str(&json::stringify(object! {
        "labels": {
            "foo": "bar"
        }
    }))
    .unwrap();

    let _foo: UpdateVersion = serde_json::from_str(&json::stringify(object! {})).unwrap();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetVersion {
    pub version: String,
    #[serde(flatten)]
    pub labels: RevisionLabels,
}

#[test]
fn validate_get_version_deserialize() {
    use json::object;

    let _foo: GetVersion = serde_json::from_str(&json::stringify(object! {
        "version":  "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
        "labels": {
            "foo": "bar"
        }
    }))
    .unwrap();

    let _foo: GetVersion = serde_json::from_str(&json::stringify(object! {
        "version":  "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
    }))
    .unwrap();
}

impl From<crate::backend::models::DataStoreRevision> for GetVersion {
    fn from(source: crate::backend::models::DataStoreRevision) -> Self {
        (&source).into()
    }
}

impl From<&crate::backend::models::DataStoreRevision> for GetVersion {
    fn from(source: &crate::backend::models::DataStoreRevision) -> Self {
        Self {
            version: source.version.clone(),
            labels: source.labels.clone(),
        }
    }
}

pub fn create_version_api(
    db: crate::Backend,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    create_version(db.clone())
        .or(update_version(db.clone()))
        .or(delete_version(db.clone()))
        .or(list_versions(db.clone()))
}

fn create_version(
    db: crate::Backend,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("POST /api/org/{{org}}/repo/{{repo}}/version");
    warp::path!("api" / "org" / String / "repo" / String / "version")
        .and(warp::post())
        .and(json_body::<CreateVersion>())
        .and(with_db(db))
        .and_then(create_version_impl)
}

#[instrument(name = "rest_version_create", fields(version = %version.version), skip(version, db))]
async fn create_version_impl(
    org: String,
    repo: String,
    version: CreateVersion,
    db: crate::Backend,
) -> Result<impl Reply, Rejection> {
    let result = db
        .create_version(&org, &repo, &version.version, version.labels.labels)
        .await;
    let result = result.map(GetVersion::from);
    wrap_body(result.map_err(ErrorStatusResponse::from))
}

fn update_version(
    db: crate::Backend,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("PUT /api/org/{{org}}/repo/{{repo}}/version/{{version}}");
    warp::path!("api" / "org" / String / "repo" / String / "version" / String)
        .and(warp::put())
        .and(json_body::<UpdateVersion>())
        .and(with_db(db))
        .and_then(update_version_impl)
}

#[instrument(name = "rest_version_update", skip(db, update))]
async fn update_version_impl(
    org: String,
    repo: String,
    version: String,
    update: UpdateVersion,
    db: crate::Backend,
) -> Result<impl Reply, Rejection> {
    let result = db
        .update_version(&org, &repo, &version, update.labels)
        .await;
    let result = result.map(GetVersion::from);
    wrap_body(result.map_err(ErrorStatusResponse::from))
}

fn delete_version(
    db: crate::Backend,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("DELETE /api/org/{{org}}/repo/{{repo}}/version/{{version}}");
    warp::path!("api" / "org" / String / "repo" / String / "version" / String)
        .and(warp::delete())
        .and(with_db(db))
        .and_then(delete_version_impl)
}

#[instrument(name = "rest_version_delete", skip(db))]
async fn delete_version_impl(
    org: String,
    repo: String,
    version: String,
    db: crate::Backend,
) -> Result<impl Reply, Rejection> {
    let result = db.delete_version(&org, &repo, &version).await;
    let result: Result<DeleteStatus, BackendError> = result.map(DeleteStatus::from);
    wrap_body(result.map_err(ErrorStatusResponse::from))
}

fn list_versions(
    db: crate::Backend,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("GET /api/org/{{org}}/repo/{{repo}}/version");
    warp::path!("api" / "org" / String / "repo" / String / "version")
        .and(warp::get())
        .and(warp::query::<ApiPagination>())
        .and(with_db(db))
        .and_then(list_versions_impl)
}

#[instrument(name = "rest_version_list", skip(db))]
async fn list_versions_impl(
    org: String,
    repo: String,
    pagination: ApiPagination,
    db: crate::Backend,
) -> Result<impl Reply, Rejection> {
    let result = db.list_versions(&org, &repo, pagination.into()).await;
    let result: Result<Vec<GetVersion>, BackendError> =
        result.map(|version_list| version_list.versions.iter().map(GetVersion::from).collect());
    wrap_body(result.map_err(ErrorStatusResponse::from))
}

#[cfg(test)]
mod integ_test {
    use super::*;
    use crate::backend::models::PaginationOptions;
    use crate::database::prelude::*;
    use crate::test_utils::*;
    use json::{array, object};
    use serial_test::serial;
    use warp::test::request;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_create_version() {
        let (backend, db) = make_db().await;
        let filter =
            create_version_api(db.clone()).recover(crate::api::canned_response::handle_rejection);

        create_org_and_repos(&backend, "example", vec!["example-repo-1"])
            .await
            .unwrap();

        let response = request()
            .path("/api/org/example/repo/example-repo-1/version")
            .body(json::stringify(object! {
                "version":  "1.2.3",
                "labels": {
                    "release_status": "pre-release"
                }
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_200_response(
            response,
            object! {
                "version":  "1.2.3",
                "labels": {
                    "release_status": "pre-release"
                }
            },
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_delete_version() {
        let (backend, db) = make_db().await;
        let filter =
            create_version_api(db.clone()).recover(crate::api::canned_response::handle_rejection);

        create_org_and_repos(&backend, "example", vec!["example-repo-1"])
            .await
            .unwrap();
        create_test_version(&backend, "example", "example-repo-1", "1.2.3")
            .await
            .unwrap();

        let response = request()
            .path("/api/org/example/repo/example-repo-1/version/1.2.3")
            .method("DELETE")
            .reply(&filter)
            .await;

        assert_200_response(
            response,
            object! {
                "deleted":  true
            },
        );

        let revisions = backend
            .list_revisions(
                &RepoParam::new("example", "example-repo-1"),
                PaginationOptions::new(0, 50),
            )
            .await
            .unwrap();
        assert_eq!(revisions.len(), 0)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_update_version_label() {
        let (backend, db) = make_db().await;
        let filter =
            create_version_api(db.clone()).recover(crate::api::canned_response::handle_rejection);

        create_org_and_repos(&backend, "example", vec!["example-repo-1"])
            .await
            .unwrap();

        let response = request()
            .path("/api/org/example/repo/example-repo-1/version")
            .body(json::stringify(object! {
                "version":  "1.2.3",
                "labels": {
                    "release_status": "pre-release"
                }
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_200_response(
            response,
            object! {
                "version":  "1.2.3",
                "labels": {
                    "release_status": "pre-release"
                }
            },
        );

        let response = request()
            .path("/api/org/example/repo/example-repo-1/version/1.2.3")
            .body(json::stringify(object! {
                "labels": {
                    "release_status": "release"
                }
            }))
            .method("PUT")
            .reply(&filter)
            .await;

        assert_200_response(
            response,
            object! {
                "version":  "1.2.3",
                "labels": {
                    "release_status": "release"
                }
            },
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_listing_versions() {
        let (backend, db) = make_db().await;
        let filter =
            create_version_api(db.clone()).recover(crate::api::canned_response::handle_rejection);

        create_org_and_repos(&backend, "example", vec!["example-repo-1"])
            .await
            .unwrap();
        for i in 1..100 {
            create_test_version(&backend, "example", "example-repo-1", &format!("1.2.{}", i))
                .await
                .unwrap()
        }

        let mut page = Vec::new();
        for i in 1..=50 {
            page.push(object!{ "version": format!("1.2.{}", i), "labels": { "version": format!("1.2.{}", i)}});
        }

        let response = request()
            .path("/api/org/example/repo/example-repo-1/version")
            .method("GET")
            .reply(&filter)
            .await;

        assert_response(response, http::StatusCode::OK, page.into());

        let mut page = Vec::new();
        for i in 51..100 {
            page.push(object!{ "version": format!("1.2.{}", i), "labels": { "version": format!("1.2.{}", i)}});
        }

        let response = request()
            .path("/api/org/example/repo/example-repo-1/version?page=1")
            .method("GET")
            .reply(&filter)
            .await;

        assert_response(response, http::StatusCode::OK, page.into());

        let response = request()
            .path("/api/org/example/repo/example-repo-1/version?page=2")
            .method("GET")
            .reply(&filter)
            .await;

        assert_response(response, http::StatusCode::OK, array![]);

        let mut page = Vec::new();
        for i in 1..=20 {
            page.push(object!{ "version": format!("1.2.{}", i), "labels": { "version": format!("1.2.{}", i)}});
        }

        let response = request()
            .path("/api/org/example/repo/example-repo-1/version?size=20")
            .method("GET")
            .reply(&filter)
            .await;

        assert_response(response, http::StatusCode::OK, page.into());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_create_version_that_will_be_rejected() {
        let (backend, db) = make_db().await;
        let filter =
            create_version_api(db.clone()).recover(crate::api::canned_response::handle_rejection);

        create_org_and_repos(&backend, "example", vec!["example-repo-1"])
            .await
            .unwrap();

        let response = request()
            .path("/api/org/example/repo/example-repo-1/version")
            .body(json::stringify(object! {
                "version":  "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_error_response(
            response,
            http::StatusCode::BAD_REQUEST, "Version string 'abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz' was more than the 30 character limit"
        );
    }
}
