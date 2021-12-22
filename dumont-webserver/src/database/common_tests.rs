#[cfg(test)]
use super::prelude::*;
#[cfg(test)]
pub use sea_orm::{entity::*, query::*, Database, DatabaseConnection};

#[cfg(test)]
pub async fn setup_schema() -> DbResult<DatabaseConnection> {
    use super::entity::prelude::*;
    use sea_orm::schema::Schema;

    let db = Database::connect("sqlite::memory:").await?;

    db.execute(
        db.get_database_backend()
            .build(&Schema::create_table_from_entity(Organization)),
    )
    .await?;

    db.execute(
        db.get_database_backend()
            .build(&Schema::create_table_from_entity(Repository)),
    )
    .await?;

    db.execute(
        db.get_database_backend()
            .build(&Schema::create_table_from_entity(RepositoryMetadata)),
    )
    .await?;

    Ok(db)
}
