use crate::database::prelude::*;
pub use sea_orm::{entity::*, query::*, Database, DatabaseConnection, DbBackend};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

pub async fn make_db() -> (PostgresDatabase, crate::Backend) {
    setup_schema().await.unwrap();
    let db = PostgresDatabase {
        db: Database::connect(&get_db_url_with_test_db()).await.unwrap(),
        date_time_provider: DateTimeProvider::RealDateTime,
    };
    let backend = Arc::new(crate::backend::DefaultBackend {
        database: db,
        policy_container: Default::default(),
    });
    let db = PostgresDatabase {
        db: Database::connect(&get_db_url_with_test_db()).await.unwrap(),
        date_time_provider: DateTimeProvider::RealDateTime,
    };
    (db, backend)
}

pub async fn setup_schema() -> DbResult<DatabaseConnection> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&get_db_host_url())
        .await?;

    let mut conn = pool.acquire().await?;

    sqlx::query!("DROP DATABASE IF EXISTS postgres_test")
        .execute(&mut conn)
        .await?;

    sqlx::query!("CREATE DATABASE postgres_test")
        .execute(&mut conn)
        .await?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&get_db_url_with_test_db())
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(Database::connect(&get_db_url_with_test_db()).await?)
}

fn get_db_host_url() -> String {
    let host = std::env::var("TEST_DB_HOST").unwrap_or("127.0.0.1".to_owned());
    format!("postgresql://postgres:password@{}:5432", host)
}

fn get_db_url_with_test_db() -> String {
    format!("{}/postgres_test", get_db_host_url())
}

#[allow(dead_code)]
pub fn logging_setup() {
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::{
        fmt::format::{Format, PrettyFields},
        layer::SubscriberExt,
        Registry,
    };

    let logger = tracing_subscriber::fmt::layer()
        .event_format(Format::default().pretty())
        .fmt_fields(PrettyFields::new());

    let subscriber = Registry::default().with(LevelFilter::DEBUG).with(logger);

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    tracing_log::LogTracer::init().expect("logging to work correctly")
}

pub async fn create_repo(db: &PostgresDatabase, org: &str, repo: &str) -> DbResult<()> {
    create_repo_with_params(db, org, repo, CreateRepoParam::default())
        .await
        .unwrap();

    Ok(())
}

pub async fn create_org_and_repos<T: ToString>(
    db: &PostgresDatabase,
    org: &str,
    repos: Vec<T>,
) -> DbResult<()> {
    db.create_org(org).await.unwrap();
    for repo in repos {
        create_repo(db, org, repo).await.unwrap();
    }

    Ok(())
}

pub async fn create_test_version(
    db: &PostgresDatabase,
    org: &str,
    repo: &str,
    version: &str,
) -> DbResult<()> {
    db.create_revision(
        &RevisionParam::new(org, repo, version),
        &CreateRevisionParam {
            artifact_url: None,
            labels: vec![("version", version)].into(),
        },
    )
    .await
    .unwrap();

    Ok(())
}
pub async fn create_repo_with_params(
    db: &PostgresDatabase,
    org: &str,
    repo: &str,
    create_param: CreateRepoParam,
) -> DbResult<()> {
    db.create_repo(&RepoParam::new(org, repo), create_param)
        .await
        .unwrap();

    Ok(())
}

pub fn assert_200_response(response: http::Response<bytes::Bytes>, expected_body: json::JsonValue) {
    use json::object;
    assert_response(
        response,
        http::StatusCode::OK,
        object! {
            "status": { "code": 200 },
            "data": expected_body
        },
    );
}

pub fn assert_200_list_response(
    response: http::Response<bytes::Bytes>,
    expected_body: json::JsonValue,
    total: usize,
    has_more: bool,
) {
    use json::object;
    assert_response(
        response,
        http::StatusCode::OK,
        object! {
            "status": { "code": 200 },
            "data": expected_body,
            "page": {
                "more": has_more,
                "total": total,
            }
        },
    );
}

pub fn assert_response(
    response: http::Response<bytes::Bytes>,
    status: http::StatusCode,
    expected_body: json::JsonValue,
) {
    let body = String::from_utf8(response.body().to_vec()).unwrap();
    println!("{:?}", body);
    let body = match json::parse(&body) {
        Err(e) => {
            println!("Unable to deserialize {:?}. Error: {:?}", body, e);
            unreachable!()
        }
        Ok(body) => body,
    };
    assert_eq!(json::stringify(body), json::stringify(expected_body));
    assert_eq!(response.status(), status);
}

pub fn assert_error_response(
    response: http::Response<bytes::Bytes>,
    status: http::StatusCode,
    message: &str,
) {
    use json::object;
    let body = String::from_utf8(response.body().to_vec()).unwrap();
    println!("{:?}", body);
    let body = match json::parse(&body) {
        Err(e) => {
            println!("Unable to deserialize {:?}. Error: {:?}", body, e);
            unreachable!()
        }
        Ok(body) => body,
    };
    assert_eq!(
        json::stringify(body),
        json::stringify(object! {
            "status": { "code": response.status().as_u16(), "error": [message] },
        })
    );
    assert_eq!(response.status(), status);
}
