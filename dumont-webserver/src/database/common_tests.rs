#[cfg(test)]
use super::prelude::*;
#[cfg(test)]
pub use sea_orm::{entity::*, query::*, Database, DatabaseConnection};

#[cfg(test)]
pub async fn setup_schema() -> DbResult<DatabaseConnection> {
    use super::entity::prelude::*;
    use sea_orm::schema::Schema;

    let db = Database::connect("sqlite::memory:").await?;
    let db_backed = db.get_database_backend();
    let schema = Schema::new(db_backed);

    db.execute(db_backed.build(&schema.create_table_from_entity(Organization)))
        .await?;

    db.execute(db_backed.build(&schema.create_table_from_entity(Repository)))
        .await?;

    db.execute(db_backed.build(&schema.create_table_from_entity(RepositoryLabel)))
        .await?;

    db.execute(db_backed.build(&schema.create_table_from_entity(RepositoryRevision)))
        .await?;

    db.execute(db_backed.build(&schema.create_table_from_entity(RepositoryRevisionLabel)))
        .await?;

    Ok(db)
}


#[cfg(test)]
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