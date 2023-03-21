pub mod metrics;
mod orgs;
mod repos;
mod versions;

use warp::{Filter, Reply};

pub async fn create_filters(
    db: crate::Backend,
) -> impl Filter<Extract = (impl Reply,)> + Clone + Send + Sync + 'static {
    filters::api(db)
        .recover(canned_response::handle_rejection)
        .with(warp::trace::request())
}

pub mod prelude {
    pub use super::models::*;
    use crate::backend::models::PaginationOptions;
    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use warp::{reject::Reject, Filter, Rejection, Reply};

    pub fn json_body<T: Send + DeserializeOwned>(
    ) -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone {
        // When accepting a body, we want a JSON body
        // (and to reject huge payloads)...
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }

    pub fn with_db(
        db: crate::Backend,
    ) -> impl Filter<Extract = (crate::Backend,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || db.clone())
    }

    pub fn wrap_body<T>(
        body: Result<PaginatedWrapperResponse<T>, impl Reject>,
    ) -> Result<impl Reply, Rejection>
    where
        T: Serialize,
    {
        let body = match body {
            Err(e) => {
                return Err(warp::reject::custom(e));
            }
            Ok(value) => value,
        };

        let response = ApplicationResponse {
            status: StatusResponse::ok(),
            data: Some(body.data),
            page: body.page_options,
        };

        Ok(warp::reply::json(&response))
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct ApiPagination {
        pub page: Option<u32>,
        pub size: Option<u32>,
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
                page_number: source.page.unwrap_or(0) as u64,
                page_size: source.size.unwrap_or(50) as u64,
            }
        }
    }

    #[derive(Deserialize, Serialize)]
    pub struct DeleteStatus {
        pub deleted: bool,
    }

    impl From<bool> for DeleteStatus {
        fn from(source: bool) -> Self {
            Self { deleted: source }
        }
    }

    pub struct PaginatedWrapperResponse<T>
    where
        T: Serialize,
    {
        data: T,
        page_options: Option<super::models::PaginationState>,
    }

    impl<T: Serialize> PaginatedWrapperResponse<T> {
        pub fn without_page(body: T) -> PaginatedWrapperResponse<T> {
            PaginatedWrapperResponse {
                data: body,
                page_options: None,
            }
        }

        pub fn with_page(body: T, total: usize, has_more: bool) -> PaginatedWrapperResponse<T> {
            PaginatedWrapperResponse {
                data: body,
                page_options: Some(PaginationState { total, has_more }),
            }
        }
    }
}

mod models {
    use serde::Serialize;
    use warp::http::StatusCode;

    #[derive(Debug, Serialize)]
    pub struct PaginationState {
        #[serde(rename = "more")]
        pub has_more: bool,
        pub total: usize,
    }

    #[derive(Serialize)]
    #[serde(remote = "StatusCode")]
    struct StatusCodeDef {
        #[serde(getter = "StatusCode::as_u16")]
        code: u16,
    }

    #[derive(Debug, Serialize)]
    #[serde(untagged)]
    pub enum StatusResponse {
        Success(SuccessfulStatusResponse),
        Error(ErrorStatusResponse),
    }

    #[derive(Debug, Serialize)]
    pub struct SuccessfulStatusResponse {
        #[serde(flatten, with = "StatusCodeDef")]
        pub code: StatusCode,
    }

    #[derive(Debug, Serialize)]
    pub struct ErrorStatusResponse {
        #[serde(flatten, with = "StatusCodeDef")]
        pub code: StatusCode,
        pub error: Option<Vec<String>>,
    }

    impl ErrorStatusResponse {
        pub fn from_error_message(code: StatusCode, error: String) -> Self {
            Self {
                code,
                error: Some(vec![error]),
            }
        }
    }

    impl StatusResponse {
        pub fn ok() -> Self {
            StatusResponse::Success(SuccessfulStatusResponse {
                code: StatusCode::OK,
            })
        }

        pub fn status(&self) -> StatusCode {
            match self {
                StatusResponse::Error(err) => err.code,
                StatusResponse::Success(suc) => suc.code,
            }
        }
    }

    #[derive(Debug, Serialize)]
    pub struct ApplicationResponse<T>
    where
        T: Serialize,
    {
        pub status: StatusResponse,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub data: Option<T>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub page: Option<PaginationState>,
    }

    #[test]
    fn validate_create_repo_deserialize() {
        use json::object;

        let serialized = serde_json::to_string(&ApplicationResponse::<()> {
            status: StatusResponse::ok(),
            data: None,
            page: None,
        })
        .unwrap();

        assert_eq!(
            serialized,
            json::stringify(object! {
                    "status": {
                        "code": 200
                    },
            })
        );
    }
}

mod filters {
    use warp::{Filter, Rejection, Reply};

    pub fn api(
        db: crate::Backend,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        super::orgs::create_org_api(db.clone())
            .or(super::repos::create_repo_api(db.clone()))
            .or(super::versions::create_version_api(db))
            .with(warp::log::custom(super::metrics::track_status))
    }
}

mod canned_response {
    use super::models::*;
    use crate::backend::BackendError;
    use crate::database::DatabaseError;
    use std::convert::Infallible;
    use std::error::Error;
    use tracing::error;
    use warp::{http::StatusCode, reject::Reject, Rejection, Reply};

    impl From<Rejection> for ErrorStatusResponse {
        fn from(source: Rejection) -> Self {
            if source.is_not_found() {
                ErrorStatusResponse::from_error_message(
                    StatusCode::NOT_FOUND,
                    "NOT_FOUND".to_owned(),
                )
            } else if let Some(resp) = source.find::<ErrorStatusResponse>() {
                resp.into()
            } else if let Some(backend_error) = source.find::<BackendError>() {
                backend_error.into()
            } else if let Some(e) = source.find::<warp::filters::body::BodyDeserializeError>() {
                let message_body: String = match e.source() {
                    Some(cause) => cause.to_string(),
                    None => "BAD_REQUEST".into(),
                };
                ErrorStatusResponse::from_error_message(StatusCode::BAD_REQUEST, message_body)
            } else if source.find::<warp::reject::MethodNotAllowed>().is_some() {
                ErrorStatusResponse::from_error_message(
                    StatusCode::METHOD_NOT_ALLOWED,
                    "METHOD_NOT_ALLOWED".to_owned(),
                )
            } else {
                error!("unhandled rejection: {:?}", source);
                ErrorStatusResponse::from_error_message(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "UNHANDLED_REJECTION".to_owned(),
                )
            }
        }
    }

    impl From<&ErrorStatusResponse> for ErrorStatusResponse {
        fn from(source: &ErrorStatusResponse) -> Self {
            Self {
                code: source.code,
                error: source.error.clone(),
            }
        }
    }

    impl From<BackendError> for ErrorStatusResponse {
        fn from(error: BackendError) -> Self {
            (&error).into()
        }
    }

    impl From<&BackendError> for ErrorStatusResponse {
        fn from(error: &BackendError) -> Self {
            let message = error.to_string();
            match error {
                BackendError::DatabaseError { source } => match source {
                    DatabaseError::NotFound { error } => ErrorStatusResponse::from_error_message(
                        StatusCode::NOT_FOUND,
                        error.to_string(),
                    ),
                    DatabaseError::AlreadyExists { error } => {
                        ErrorStatusResponse::from_error_message(
                            StatusCode::CONFLICT,
                            error.to_string(),
                        )
                    }
                    _ => {
                        error!("Internal Error: {}", source);
                        ErrorStatusResponse::from_error_message(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            message,
                        )
                    }
                },
                BackendError::ConstraintViolation { reason } => {
                    ErrorStatusResponse::from_error_message(
                        StatusCode::BAD_REQUEST,
                        reason.to_string(),
                    )
                }
                BackendError::PolicyViolation { error } => ErrorStatusResponse::from_error_message(
                    StatusCode::BAD_REQUEST,
                    error.to_string(),
                ),
            }
        }
    }

    impl Reject for ErrorStatusResponse {}

    pub async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
        let status = StatusResponse::Error(err.into());
        let status_code = status.status();
        let response: ApplicationResponse<()> = ApplicationResponse {
            data: None,
            status,
            page: None,
        };

        let json = warp::reply::json(&response);

        Ok(warp::reply::with_status(json, status_code))
    }
}
