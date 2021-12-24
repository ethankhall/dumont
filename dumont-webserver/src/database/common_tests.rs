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

    db.execute(db_backed.build(&schema.create_table_from_entity(RepositoryMetadata)))
        .await?;

    Ok(db)
}
