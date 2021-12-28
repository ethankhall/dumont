use super::canned_response::ApplicationError;
use super::prelude::*;
use crate::backend::BackendError;
use tracing::info;
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
    db: crate::Db,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    create_org(db.clone())
        .or(delete_org(db.clone()))
        .or(list_orgs(db.clone()))
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

async fn create_org_impl(org: CreateOrganization, db: crate::Db) -> Result<impl Reply, Rejection> {
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

async fn delete_org_impl(org_name: String, db: crate::Db) -> Result<impl Reply, Rejection> {
    let result = db.delete_organization(&org_name).await;
    let result: Result<DeleteStatus, BackendError> = result.map(DeleteStatus::from);
    wrap_body(result.map_err(ApplicationError::from_context))
}

fn list_orgs(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("GET /api/org");
    warp::path!("api" / "org")
        .and(warp::get())
        .and(warp::query::<ApiPagination>())
        .and(with_db(db))
        .and_then(list_orgs_impl)
}

async fn list_orgs_impl(
    pageination: ApiPagination,
    db: crate::Db,
) -> Result<impl Reply, Rejection> {
    let result = db.get_organizations(pageination.into()).await;
    let result: Result<DataWrapper<Vec<GetOrganization>>, BackendError> = result.map(|orgs_list| {
        DataWrapper::new(orgs_list.orgs.iter().map(GetOrganization::from).collect())
    });
    wrap_body(result.map_err(ApplicationError::from_context))
}

fn get_an_org(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    info!("GET /api/org/{{org}}");
    warp::path!("api" / "org" / String)
        .and(warp::get())
        .and(with_db(db))
        .and_then(get_an_org_impl)
}

async fn get_an_org_impl(org_name: String, db: crate::Db) -> Result<impl Reply, Rejection> {
    let result = db.get_organization(&org_name).await;
    let result: Result<GetOrganization, BackendError> =
        result.map(|org| GetOrganization::from(org));
    wrap_body(result.map_err(ApplicationError::from_context))
}

#[cfg(test)]
mod integ_test {
    use super::*;
    use crate::test_utils::*;
    use json::object;
    use serial_test::serial;
    use warp::test::request;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_create_org() {
        let (_backend, db) = make_db().await;
        let filter = create_org(db.clone());

        let response = request()
            .path("/api/org")
            .body(json::stringify(object! {
                "org": "example-org"
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);
        let body = json::parse(&String::from_utf8(response.body().to_vec()).unwrap()).unwrap();
        assert_eq!(body, object! {"org":  "example-org"});
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_duplicate_org() {
        let (_backend, db) = make_db().await;
        let filter = create_org(db.clone()).recover(crate::api::canned_response::handle_rejection);

        let response = request()
            .path("/api/org")
            .body(json::stringify(object! {
                "org": "example-org"
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);
        let body = json::parse(&String::from_utf8(response.body().to_vec()).unwrap()).unwrap();
        assert_eq!(body, object! {"org":  "example-org"});

        let response = request()
            .path("/api/org")
            .body(json::stringify(object! {
                "org": "example-org"
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 409);
        let body = json::parse(&String::from_utf8(response.body().to_vec()).unwrap()).unwrap();
        assert_eq!(
            body,
            object! {
                "code": 409,
                "message": "Org example-org exists"
            }
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_list_org() {
        let (_backend, db) = make_db().await;
        let filter = create_org(db.clone()).recover(crate::api::canned_response::handle_rejection);

        let response = request()
            .path("/api/org")
            .body(json::stringify(object! {
                "org": "example-org"
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);
        let body = json::parse(&String::from_utf8(response.body().to_vec()).unwrap()).unwrap();
        assert_eq!(body, object! {"org":  "example-org"});

        let response = request()
            .path("/api/org")
            .body(json::stringify(object! {
                "org": "example-org-2"
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);
        let body = json::parse(&String::from_utf8(response.body().to_vec()).unwrap()).unwrap();
        assert_eq!(body, object! {"org":  "example-org-2"});

        let filter = list_orgs(db.clone()).recover(crate::api::canned_response::handle_rejection);

        let response = request()
            .path("/api/org")
            .method("GET")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);
        let body = json::parse(&String::from_utf8(response.body().to_vec()).unwrap()).unwrap();
        assert_eq!(
            body,
            object! {"data":  [{"org": "example-org"}, {"org": "example-org-2"}]}
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_delete_org() {
        let (_backend, db) = make_db().await;
        let filter = create_org(db.clone()).recover(crate::api::canned_response::handle_rejection);

        let response = request()
            .path("/api/org")
            .body(json::stringify(object! {
                "org": "example-org"
            }))
            .method("POST")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);
        let body = json::parse(&String::from_utf8(response.body().to_vec()).unwrap()).unwrap();
        assert_eq!(body, object! {"org":  "example-org"});

        let filter = delete_org(db.clone()).recover(crate::api::canned_response::handle_rejection);
        let response = request()
            .path("/api/org/example-org")
            .method("DELETE")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);
        let body = json::parse(&String::from_utf8(response.body().to_vec()).unwrap()).unwrap();
        assert_eq!(
            body,
            object! {
                "deleted": true,
            }
        );

        let filter = list_orgs(db.clone()).recover(crate::api::canned_response::handle_rejection);

        let response = request()
            .path("/api/org")
            .method("GET")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);
        let body = json::parse(&String::from_utf8(response.body().to_vec()).unwrap()).unwrap();
        assert_eq!(body, object! {"data":  []});
    }
}
