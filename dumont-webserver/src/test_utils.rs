use crate::database::prelude::*;
pub use sea_orm::{entity::*, query::*, Database, DatabaseConnection, DbBackend};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

pub async fn make_db() -> (PostgresDatabase, crate::Db) {
    setup_schema().await.unwrap();
    let db = PostgresDatabase {
        db: Database::connect("postgresql://postgres:password@127.0.0.1:5432/postgres_test")
            .await
            .unwrap(),
        date_time_provider: DateTimeProvider::RealDateTime,
    };
    let backend = Arc::new(crate::backend::DefaultBackend { database: db });
    let db = PostgresDatabase {
        db: Database::connect("postgresql://postgres:password@127.0.0.1:5432/postgres_test")
            .await
            .unwrap(),
        date_time_provider: DateTimeProvider::RealDateTime,
    };
    (db, backend)
}

pub async fn setup_schema() -> DbResult<DatabaseConnection> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgresql://postgres:password@127.0.0.1:5432/")
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
        .connect("postgresql://postgres:password@127.0.0.1:5432/postgres_test")
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(Database::connect("postgresql://postgres:password@127.0.0.1:5432/postgres_test").await?)
}

#[allow(dead_code)]
pub fn logging_setup() -> () {
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::{
        fmt::format::{Format, PrettyFields},
        layer::SubscriberExt,
        Registry,
    };

    let logger = tracing_subscriber::fmt::layer()
        .event_format(Format::default().pretty())
        .fmt_fields(PrettyFields::new());

    let subscriber = Registry::default()
        .with(LevelFilter::from(LevelFilter::DEBUG))
        .with(logger);

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    tracing_log::LogTracer::init().expect("logging to work correctly")
}

pub async fn create_repo(db: &PostgresDatabase, org: &str, repo: &str) -> DbResult<()> {
    create_repo_with_params(db, org, repo, CreateRepoParam::default())
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
