mod orgs;
mod repos;
mod versions;

use warp::{Filter, Reply};

pub async fn create_filters(
    db: crate::Db,
) -> impl Filter<Extract = impl Reply> + Clone + Send + Sync + 'static {
    filters::api(db).recover(canned_response::handle_rejection)
}

pub mod prelude {
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
        db: crate::Db,
    ) -> impl Filter<Extract = (crate::Db,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || db.clone())
    }

    pub fn wrap_body<T>(body: Result<T, impl Reject>) -> Result<impl Reply, Rejection>
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

    #[derive(Deserialize, Serialize)]
    pub struct DataWrapper<T: Serialize> {
        pub data: T,
    }

    impl<T: Serialize> DataWrapper<T> {
        pub fn new(data: T) -> Self {
            Self { data }
        }
    }

    #[derive(Deserialize, Serialize)]
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
                page_number: source.page.unwrap_or(0) as usize,
                page_size: source.size.unwrap_or(50) as usize,
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
}

mod filters {
    use warp::{Filter, Rejection, Reply};

    pub fn api(db: crate::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        super::orgs::create_org_api(db.clone())
            .or(super::repos::create_repo_api(db.clone()))
            .or(super::versions::create_version_api(db.clone()))
    }
}

mod canned_response {
    use crate::backend::BackendError;
    use crate::database::DatabaseError;
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
                BackendError::DatabaseError { source } => match source {
                    DatabaseError::NotFound { error } => Self {
                        code: StatusCode::NOT_FOUND,
                        message: error.to_string(),
                    },
                    DatabaseError::AlreadyExists { error } => Self {
                        code: StatusCode::CONFLICT,
                        message: error.to_string(),
                    },
                    _ => {
                        error!("Internal Error: {}", source);
                        Self {
                            code: StatusCode::INTERNAL_SERVER_ERROR,
                            message,
                        }
                    }
                },
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
