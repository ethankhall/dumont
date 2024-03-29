use super::prelude::*;
use tracing::info;
use tracing_attributes::instrument;
use warp::{Filter, Rejection, Reply};

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

pub fn create_org_api(
    db: crate::Backend,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    create_org(db.clone())
        .or(delete_org(db.clone()))
        .or(list_orgs(db.clone()))
        .or(get_an_org(db))
}

fn create_org(
    db: crate::Backend,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    info!("POST /api/org");
    warp::path!("api" / "org")
        .and(warp::post())
        .and(json_body::<CreateOrganization>())
        .and(with_db(db))
        .and_then(create_org_impl)
}

#[instrument(name = "rest_org_create", skip(db))]
async fn create_org_impl(
    org: CreateOrganization,
    db: crate::Backend,
) -> Result<impl Reply, Rejection> {
    let result = db.create_organization(&org.org).await;
    let result = result
        .map(|org| GetOrganization { org: org.name })
        .map(PaginatedWrapperResponse::without_page)
        .map_err(ErrorStatusResponse::from);
    wrap_body(result)
}

fn delete_org(
    db: crate::Backend,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    info!("DELETE /api/org/{{org}}");
    warp::path!("api" / "org" / String)
        .and(warp::delete())
        .and(with_db(db))
        .and_then(delete_org_impl)
}

#[instrument(name = "rest_org_delete", skip(db))]
async fn delete_org_impl(org_name: String, db: crate::Backend) -> Result<impl Reply, Rejection> {
    let result = db.delete_organization(&org_name).await;
    let result = result
        .map(DeleteStatus::from)
        .map(PaginatedWrapperResponse::without_page)
        .map_err(ErrorStatusResponse::from);
    wrap_body(result)
}

fn list_orgs(
    db: crate::Backend,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    info!("GET /api/org");
    warp::path!("api" / "org")
        .and(warp::get())
        .and(warp::query::<ApiPagination>())
        .and(with_db(db))
        .and_then(list_orgs_impl)
}

#[instrument(name = "rest_org_list", skip(db))]
async fn list_orgs_impl(
    pageination: ApiPagination,
    db: crate::Backend,
) -> Result<impl Reply, Rejection> {
    let result = db.list_organizations(pageination.into()).await;
    let result: Result<PaginatedWrapperResponse<Vec<GetOrganization>>, ErrorStatusResponse> =
        result
            .map(|orgs_list| {
                (
                    orgs_list.orgs.iter().map(GetOrganization::from).collect(),
                    orgs_list.total_count,
                    orgs_list.has_more,
                )
            })
            .map(|(body, total_count, has_more)| {
                PaginatedWrapperResponse::with_page(body, total_count, has_more)
            })
            .map_err(ErrorStatusResponse::from);
    wrap_body(result)
}

fn get_an_org(
    db: crate::Backend,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    info!("GET /api/org/{{org}}");
    warp::path!("api" / "org" / String)
        .and(warp::get())
        .and(with_db(db))
        .and_then(get_an_org_impl)
}

#[instrument(name = "rest_org_get", skip(db))]
async fn get_an_org_impl(org_name: String, db: crate::Backend) -> Result<impl Reply, Rejection> {
    let result = db.get_organization(&org_name).await;
    let result = result
        .map(GetOrganization::from)
        .map(PaginatedWrapperResponse::without_page)
        .map_err(ErrorStatusResponse::from);
    wrap_body(result)
}

#[cfg(test)]
mod integ_test {
    use super::*;
    use crate::test_utils::*;
    use json::{array, object};
    use serial_test::serial;
    use warp::test::request;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_create_org() {
        let backend = make_backend().await;
        let filter = create_org(backend.clone());

        let response = request()
            .path("/api/org")
            .body(json::stringify(object! {
                "org": "example-org"
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_200_response(response, object! {"org":  "example-org"});
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_duplicate_org() {
        let backend = make_backend().await;
        let filter =
            create_org(backend.clone()).recover(crate::api::canned_response::handle_rejection);

        let response = request()
            .path("/api/org")
            .body(json::stringify(object! {
                "org": "example-org"
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_200_response(response, object! {"org":  "example-org"});

        let response = request()
            .path("/api/org")
            .body(json::stringify(object! {
                "org": "example-org"
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_error_response(
            response,
            http::StatusCode::CONFLICT,
            "Org example-org exists",
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_list_org() {
        let backend = make_backend().await;
        let filter =
            create_org(backend.clone()).recover(crate::api::canned_response::handle_rejection);

        let response = request()
            .path("/api/org")
            .body(json::stringify(object! {
                "org": "example-org"
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_200_response(response, object! {"org":  "example-org"});

        let response = request()
            .path("/api/org")
            .body(json::stringify(object! {
                "org": "example-org-2"
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_200_response(response, object! {"org":  "example-org-2"});

        let filter =
            list_orgs(backend.clone()).recover(crate::api::canned_response::handle_rejection);

        let response = request()
            .path("/api/org")
            .method("GET")
            .reply(&filter)
            .await;

        assert_200_list_response(
            response,
            array!({"org": "example-org"}, {"org": "example-org-2"}),
            2,
            false,
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_delete_org() {
        let backend = make_backend().await;
        let filter =
            create_org(backend.clone()).recover(crate::api::canned_response::handle_rejection);

        let response = request()
            .path("/api/org")
            .body(json::stringify(object! {
                "org": "example-org"
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_200_response(response, object! {"org":  "example-org"});

        let filter =
            delete_org(backend.clone()).recover(crate::api::canned_response::handle_rejection);
        let response = request()
            .path("/api/org/example-org")
            .method("DELETE")
            .reply(&filter)
            .await;

        assert_200_response(
            response,
            object! {
                "deleted": true,
            },
        );

        let filter =
            list_orgs(backend.clone()).recover(crate::api::canned_response::handle_rejection);

        let response = request()
            .path("/api/org")
            .method("GET")
            .reply(&filter)
            .await;

        assert_200_list_response(response, array! {}, 0, false);
    }
}
