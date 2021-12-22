use super::Db;
use warp::Filter;

mod operations;

pub async fn create_filters(
    db: Db,
) -> impl Filter<Extract = impl warp::Reply> + Clone + Send + Sync + 'static {
    filters::api(db)
        .recover(canned_response::handle_rejection)
        .with(warp::trace::request())
}

mod filters {
    use super::canned_response::ApplicationError;
    use super::operations::{
        CreateOrganization, CreateRepository, GetOrganization, GetRepository, VersionScheme,
    };
    use crate::backend::{models::PaginationOptions, BackendError};
    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use tracing::info;
    use warp::{reject::Reject, Filter, Rejection, Reply};

    #[derive(Deserialize, Serialize)]
    struct ApiPagination {
        page: Option<u32>,
        size: Option<u32>,
    }

    impl Default for ApiPagination {
        fn default() -> Self {
            Self {
                page: Some(0),
                size: Some(50),
            }
        }
    }

    impl From<ApiPagination> for PaginationOptions {
        fn from(source: ApiPagination) -> Self {
            Self {
                page_number: source.page.unwrap_or(0) as usize,
                page_size: source.size.unwrap_or(50) as usize,
            }
        }
    }

    fn wrap_body<T>(body: Result<T, impl Reject>) -> Result<impl Reply, Rejection>
    where
        T: Serialize,
    {
        let body = match body {
            Err(e) => {
                return Err(warp::reject::custom(e));
            }
            Ok(value) => value,
        };

        Ok(warp::reply::json(&body))
    }

    pub fn api(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        create_org(db.clone())
            .or(get_orgs(db.clone()))
            .or(create_repo(db.clone()))
            .or(get_repos(db.clone()))
            .or(get_repo(db.clone()))
    }

    fn create_org(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        info!("POST /api/orgs");
        warp::path!("api" / "orgs")
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

    fn get_orgs(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        info!("GET /api/orgs");
        warp::path!("api" / "orgs")
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
        let result: Result<Vec<GetOrganization>, BackendError> =
            result.map(|orgs_list| orgs_list.orgs.iter().map(GetOrganization::from).collect());
        wrap_body(result.map_err(ApplicationError::from_context))
    }

    fn create_repo(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        info!("POST /api/orgs/{{org}}/repos");
        warp::path!("api" / "orgs" / String / "repos")
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
        let result = db.create_repo(&org, &repo.repo, &repo.url).await;
        let result = result.map(GetRepository::from);
        wrap_body(result.map_err(ApplicationError::from_context))
    }

    fn get_repos(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        info!("GET /api/orgs/{{org}}/repos");
        warp::path!("api" / "orgs" / String / "repos")
            .and(warp::get())
            .and(with_db(db))
            .and_then(get_repos_impl)
    }

    async fn get_repos_impl(org: String, db: crate::Db) -> Result<impl Reply, Rejection> {
        let result = db.get_repos(&org).await;
        let result: Result<Vec<GetRepository>, BackendError> =
            result.map(|repo_list| repo_list.repos.iter().map(GetRepository::from).collect());
        wrap_body(result.map_err(ApplicationError::from_context))
    }

    fn get_repo(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        info!("GET /api/repos/{{org}}/{{repo}}");
        warp::path!("api" / "repos" / String / String)
            .and(warp::get())
            .and(with_db(db))
            .and_then(get_repo_impl)
    }

    async fn get_repo_impl(
        org: String,
        repo: String,
        db: crate::Db,
    ) -> Result<impl Reply, Rejection> {
        let result = db.get_repo(&org, &repo).await;
        let result: Result<GetRepository, BackendError> = result.map(GetRepository::from);
        wrap_body(result.map_err(ApplicationError::from_context))
    }

    fn json_body<T: Send + DeserializeOwned>(
    ) -> impl Filter<Extract = (T,), Error = Rejection> + Clone {
        // When accepting a body, we want a JSON body
        // (and to reject huge payloads)...
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }

    fn with_db(
        db: crate::Db,
    ) -> impl Filter<Extract = (crate::Db,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || db.clone())
    }
}

mod canned_response {
    use crate::backend::BackendError;
    use serde::Serialize;
    use serde_json::Value as JsonValue;
    use std::convert::Infallible;
    use std::error::Error;
    use tracing::error;
    use warp::{http::StatusCode, reject::Reject, Rejection, Reply};

    #[derive(Debug)]
    pub struct ApplicationError {
        pub code: StatusCode,
        pub message: String,
    }

    impl ApplicationError {
        pub fn from_context(error: BackendError) -> Self {
            let message = error.to_string();
            match error {
                BackendError::NotFound { id: _ } => Self {
                    code: StatusCode::NOT_FOUND,
                    message,
                },
                BackendError::DatabaseError { source } => {
                    error!("Internal Error: {}", source);
                    Self {
                        code: StatusCode::INTERNAL_SERVER_ERROR,
                        message,
                    }
                }
            }
        }
    }

    impl Reject for ApplicationError {}

    #[derive(Serialize)]
    struct ErrorMessage {
        pub code: u16,
        pub message: JsonValue,
    }

    pub async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
        if err.is_not_found() {
            return to_error_message(StatusCode::NOT_FOUND, serde_json::json!("NOT_FOUND"));
        } else if let Some(response) = err.find::<ApplicationError>() {
            return to_error_message(response.code, serde_json::json!(response.message));
        } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
            let message_body: String = match e.source() {
                Some(cause) => cause.to_string(),
                None => "BAD_REQUEST".into(),
            };
            return to_error_message(StatusCode::BAD_REQUEST, serde_json::json!(message_body));
        } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
            return to_error_message(
                StatusCode::METHOD_NOT_ALLOWED,
                serde_json::json!("METHOD_NOT_ALLOWED"),
            );
        } else {
            error!("unhandled rejection: {:?}", err);
            return to_error_message(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!("UNHANDLED_REJECTION"),
            );
        }
    }

    fn to_error_message(
        code: StatusCode,
        message: JsonValue,
    ) -> std::result::Result<impl Reply, Infallible> {
        let response = ErrorMessage {
            code: code.as_u16(),
            message,
        };

        let json = warp::reply::json(&response);

        Ok(warp::reply::with_status(json, code))
    }
}
