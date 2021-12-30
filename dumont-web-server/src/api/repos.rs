use super::prelude::*;
use crate::backend::BackendError;
use tracing::info;
use tracing_attributes::instrument;
use warp::{Filter, Rejection, Reply};

use serde::{Deserialize, Serialize};

type RepoLabels = crate::models::GenericLabels;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRepository {
    pub repo: String,
    #[serde(flatten, default)]
    pub labels: RepoLabels,
}

#[test]
fn validate_create_repo_deserialize() {
    use json::object;

    let _foo: CreateRepository = serde_json::from_str(&json::stringify(object! {
        "repo":  "example",
    }))
    .unwrap();

    let _foo: CreateRepository = serde_json::from_str(&json::stringify(object! {
        "repo":  "example",
        "labels": {
            "foo": "bar"
        }
    }))
    .unwrap();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetRepository {
    pub org: String,
    pub repo: String,
    #[serde(flatten)]
    pub labels: RepoLabels,
}

#[test]
fn validate_get_repo_deserialize() {
    use json::object;

    let _foo: GetRepository = serde_json::from_str(&json::stringify(object! {
        "org": "foo",
        "repo":  "example",
    }))
    .unwrap();

    let _foo: GetRepository = serde_json::from_str(&json::stringify(object! {
        "org": "foo",
        "repo":  "example",
        "labels": {
            "foo": "bar"
        }
    }))
    .unwrap();
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
    #[serde(flatten, default)]
    pub labels: RepoLabels,
}

pub fn create_repo_api(
    db: crate::Backend,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    create_repo(db.clone())
        .or(get_repos(db.clone()))
        .or(get_repo(db.clone()))
        .or(delete_repo(db.clone()))
        .or(update_repo(db.clone()))
}

fn create_repo(db: crate::Backend) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("POST /api/org/{{org}}/repo");
    warp::path!("api" / "org" / String / "repo")
        .and(warp::post())
        .and(json_body::<CreateRepository>())
        .and(with_db(db))
        .and_then(create_repo_impl)
}

#[instrument(name = "rest_org_create", fields(repo = %repo.repo), skip(repo, db))]
async fn create_repo_impl(
    org: String,
    repo: CreateRepository,
    db: crate::Backend,
) -> Result<impl Reply, Rejection> {
    let result = db.create_repo(&org, &repo.repo, repo.labels.labels).await;
    let result = result.map(GetRepository::from);
    wrap_body(result.map_err(ErrorStatusResponse::from))
}

fn get_repos(db: crate::Backend) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("GET /api/org/{{org}}/repo");
    warp::path!("api" / "org" / String / "repo")
        .and(warp::get())
        .and(warp::query::<ApiPagination>())
        .and(with_db(db))
        .and_then(get_repos_impl)
}

#[instrument(name = "rest_org_list", skip(db))]
async fn get_repos_impl(
    org: String,
    pagination: ApiPagination,
    db: crate::Backend,
) -> Result<impl Reply, Rejection> {
    let result = db.list_repos(&org, pagination.into()).await;
    let result: Result<Vec<GetRepository>, BackendError> =
        result.map(|repo_list| repo_list.repos.iter().map(GetRepository::from).collect());
    wrap_body(result.map_err(ErrorStatusResponse::from))
}

fn get_repo(db: crate::Backend) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("GET /api/org/{{org}}/repo/{{repo}}");
    warp::path!("api" / "org" / String / "repo" / String)
        .and(warp::get())
        .and(with_db(db))
        .and_then(get_repo_impl)
}

#[instrument(name = "rest_org_get", skip(db))]
async fn get_repo_impl(
    org: String,
    repo: String,
    db: crate::Backend,
) -> Result<impl Reply, Rejection> {
    let result = db.get_repo(&org, &repo).await;
    let result: Result<GetRepository, BackendError> = result.map(GetRepository::from);
    wrap_body(result.map_err(ErrorStatusResponse::from))
}

fn delete_repo(db: crate::Backend) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("DELETE /api/org/{{org}}/repo/{{repo}}");
    warp::path!("api" / "org" / String / "repo" / String)
        .and(warp::delete())
        .and(with_db(db))
        .and_then(delete_repo_impl)
}

#[instrument(name = "rest_org_delete", skip(db))]
async fn delete_repo_impl(
    org: String,
    repo: String,
    db: crate::Backend,
) -> Result<impl Reply, Rejection> {
    let result = db.delete_repo(&org, &repo).await;
    let result: Result<DeleteStatus, BackendError> = result.map(DeleteStatus::from);
    wrap_body(result.map_err(ErrorStatusResponse::from))
}

fn update_repo(db: crate::Backend) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("PUT /api/org/{{org}}/repo/{{repo}}");
    warp::path!("api" / "org" / String / "repo" / String)
        .and(warp::put())
        .and(json_body::<UpdateRepository>())
        .and(with_db(db))
        .and_then(update_repo_impl)
}

#[instrument(name = "rest_org_update", skip(db, update))]
async fn update_repo_impl(
    org: String,
    repo: String,
    update: UpdateRepository,
    db: crate::Backend,
) -> Result<impl Reply, Rejection> {
    let result = db.update_repo(&org, &repo, update.labels.labels).await;
    let result: Result<GetRepository, BackendError> = result.map(GetRepository::from);
    wrap_body(result.map_err(ErrorStatusResponse::from))
}

#[cfg(test)]
mod integ_test {
    use super::*;
    use crate::database::prelude::*;
    use crate::test_utils::*;
    use json::object;
    use serial_test::serial;
    use warp::test::request;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_create_repo() {
        let (backend, db) = make_db().await;
        let filter =
            create_repo_api(db.clone()).recover(crate::api::canned_response::handle_rejection);

        backend.create_org("example").await.unwrap();

        let response = request()
            .path("/api/org/example/repo")
            .body(json::stringify(object! {
                "repo":  "example-repo-1",
                "labels": {
                    "scm_url": "https://github.com/example/example-repo-1"
                }
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_200_response(
            response,
            object! {
                "org":  "example",
                "repo":  "example-repo-1",
                "labels": {
                    "scm_url": "https://github.com/example/example-repo-1"
                }
            },
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_update_labels() {
        let (backend, db) = make_db().await;
        let filter =
            create_repo_api(db.clone()).recover(crate::api::canned_response::handle_rejection);

        backend.create_org("example").await.unwrap();
        create_repo_with_params(
            &backend,
            "example",
            "example-repo-1",
            CreateRepoParam {
                labels: vec![("scm_url", "https://github.com/example/example-repo-1")].into(),
            },
        )
        .await
        .unwrap();

        let response = request()
            .path("/api/org/example/repo/example-repo-1")
            .method("GET")
            .reply(&filter)
            .await;

        assert_200_response(
            response,
            object! {
                "org":  "example",
                "repo":  "example-repo-1",
                "labels": {
                    "scm_url": "https://github.com/example/example-repo-1"
                }
            },
        );

        let response = request()
            .path("/api/org/example/repo/example-repo-1")
            .body(json::stringify(object! {
                "labels": {
                    "scm_url": "https://example.com/example-repo-1"
                }
            }))
            .method("PUT")
            .reply(&filter)
            .await;

        assert_200_response(
            response,
            object! {
                "org":  "example",
                "repo":  "example-repo-1",
                "labels": {
                    "scm_url": "https://example.com/example-repo-1"
                }
            },
        );

        let response = request()
            .path("/api/org/example/repo/example-repo-1")
            .method("GET")
            .reply(&filter)
            .await;

        assert_200_response(
            response,
            object! {
                "org":  "example",
                "repo":  "example-repo-1",
                "labels": {
                    "scm_url": "https://example.com/example-repo-1"
                }
            },
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_delete_repo() {
        let (backend, db) = make_db().await;
        let filter =
            create_repo_api(db.clone()).recover(crate::api::canned_response::handle_rejection);

        backend.create_org("example").await.unwrap();
        create_repo_with_params(
            &backend,
            "example",
            "example-repo-1",
            CreateRepoParam {
                labels: vec![("scm_url", "https://github.com/example/example-repo-1")].into(),
            },
        )
        .await
        .unwrap();

        backend
            .create_revision(
                &RevisionParam::new("example", "example-repo-1", "1.2.3"),
                &CreateRevisionParam {
                    artifact_url: None,
                    labels: RevisionLabels::default(),
                },
            )
            .await
            .unwrap();

        let response = request()
            .path("/api/org/example/repo/example-repo-1")
            .method("DELETE")
            .reply(&filter)
            .await;

        assert_200_response(
            response,
            object! {
                "deleted":  true
            },
        );

        let response = request()
            .path("/api/org/example/repo/example-repo-1")
            .method("GET")
            .reply(&filter)
            .await;

        assert_error_response(
            response,
            http::StatusCode::NOT_FOUND,
            "Repo example/example-repo-1 not found",
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_get_non_existent_repo() {
        let (backend, db) = make_db().await;
        let filter =
            create_repo_api(db.clone()).recover(crate::api::canned_response::handle_rejection);

        backend.create_org("example").await.unwrap();

        let response = request()
            .path("/api/org/example/repo/example-repo-1")
            .method("GET")
            .reply(&filter)
            .await;

        assert_error_response(
            response,
            http::StatusCode::NOT_FOUND,
            "Repo example/example-repo-1 not found",
        );
    }
}