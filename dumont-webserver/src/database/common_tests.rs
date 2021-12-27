use super::prelude::*;
pub use sea_orm::{DbBackend, entity::*, query::*, Database, DatabaseConnection};
use sqlx::postgres::PgPoolOptions;

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
