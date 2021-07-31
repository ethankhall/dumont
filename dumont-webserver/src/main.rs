use clap::Clap;
use dumont_backend::DataStore;
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber::{fmt::format::FmtSpan, FmtSubscriber};
use warp::Filter;

pub type Db = Arc<Box<dyn dumont_backend::DataStore>>;

#[derive(Clap, Debug)]
#[clap(author, about, version)]
pub struct Opts {
    #[clap(subcommand)]
    pub datastore_target: DataStoreTarget,
}

#[derive(Clap, Debug)]
pub enum DataStoreTarget {
    /// Create an in-memory database
    #[clap(name = "mem")]
    Memory(MemoryArgs),
}

impl DataStoreTarget {
    async fn into_backend(self) -> Db {
        match &self {
            DataStoreTarget::Memory(args) => Arc::new(args.as_backend().await),
        }
    }
}

#[derive(Clap, Debug)]
pub struct MemoryArgs {
}

impl MemoryArgs {
    async fn as_backend(&self) -> Box<dyn DataStore> {
        use dumont_backend::MemDataStore;
        return Box::new(MemDataStore::default());
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let opt = Opts::parse();

    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::INFO)
        // Record an event when each span closes. This can be used to time our
        // routes' durations!
        .with_span_events(FmtSpan::CLOSE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let db = opt.datastore_target.into_backend().await;

    let filters = filters::api(db)
        .with(warp::trace::request())
        .recover(canned_response::handle_rejection);

    warp::serve(filters).run(([127, 0, 0, 1], 3030)).await;
}

mod filters {
    use super::canned_response::EntityCretionFailed;
    use dumont_models::operations::CreateOrganization;
    use serde::de::DeserializeOwned;
    use warp::Rejection;
    use warp::{http::StatusCode, Filter};

    pub async fn create_org_impl(
        org: CreateOrganization,
        db: crate::Db,
    ) -> Result<impl warp::Reply, Rejection> {
        let _root_entity = match db.create_organization(&org).await {
            Err(e) => {
                return Err(warp::reject::custom(EntityCretionFailed {
                    message: e.to_string(),
                }))
            }
            Ok(value) => value,
        };

        Ok(StatusCode::OK)
    }

    pub fn api(
        db: crate::Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        create_org(db.clone()).or(get_orgs(db))
    }

    fn create_org(
        db: crate::Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "organization")
            .and(warp::post())
            .and(json_body::<CreateOrganization>())
            .and(with_db(db))
            .and_then(create_org_impl)
    }

    fn get_orgs(
        db: crate::Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "organization")
            .and(warp::get())
            .and(with_db(db))
            .and_then(get_orgs_impl)
    }

    pub async fn get_orgs_impl(db: crate::Db) -> Result<impl warp::Reply, Rejection> {
        let all_orgs = match db.get_organizations().await {
            Err(e) => {
                return Err(warp::reject::custom(EntityCretionFailed {
                    message: e.to_string(),
                }))
            }
            Ok(value) => value,
        };

        Ok(warp::reply::json(&all_orgs))
    }

    fn json_body<T: Send + DeserializeOwned>(
    ) -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone {
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
    use serde::Serialize;
    use serde_json::Value as JsonValue;
    use std::convert::Infallible;
    use std::error::Error;
    use tracing::error;
    use warp::{http::StatusCode, reject::Reject};

    #[derive(Debug, Serialize)]
    pub struct EntityNotFound {
        pub id: String,
    }

    impl Reject for EntityNotFound {}

    #[derive(Debug, Serialize)]
    pub struct EntityCretionFailed {
        pub message: String,
    }

    impl Reject for EntityCretionFailed {}

    #[derive(Serialize)]
    struct ErrorMessage {
        pub code: u16,
        pub message: JsonValue,
    }

    pub async fn handle_rejection(
        err: warp::Rejection,
    ) -> std::result::Result<impl warp::Reply, Infallible> {
        if err.is_not_found() {
            return to_error_message(StatusCode::NOT_FOUND, serde_json::json!("NOT_FOUND"));
        } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
            let message_body: String = match e.source() {
                Some(cause) => cause.to_string(),
                None => "BAD_REQUEST".into(),
            };
            return to_error_message(StatusCode::BAD_REQUEST, serde_json::json!(message_body));
        } else if let Some(e) = err.find::<EntityNotFound>() {
            return to_error_message(StatusCode::NOT_FOUND, serde_json::to_value(e).unwrap());
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
    ) -> std::result::Result<impl warp::Reply, Infallible> {
        let response = ErrorMessage {
            code: code.as_u16(),
            message,
        };

        let json = warp::reply::json(&response);

        Ok(warp::reply::with_status(json, code))
    }
}
