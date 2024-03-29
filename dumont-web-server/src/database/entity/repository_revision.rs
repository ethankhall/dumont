//! SeaORM Entity. Generated by sea-orm-codegen 0.6.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "repository_revision")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub revision_id: i32,
    pub repo_id: i32,
    #[sea_orm(column_type = "Text")]
    pub revision_name: String,
    pub created_at: DateTimeUtc,
    #[sea_orm(column_type = "Text", nullable)]
    pub artifact_url: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::repository::Entity",
        from = "Column::RepoId",
        to = "super::repository::Column::RepoId",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Repository,
    #[sea_orm(has_many = "super::repository_revision_label::Entity")]
    RepositoryRevisionLabel,
}

impl Related<super::repository::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Repository.def()
    }
}

impl Related<super::repository_revision_label::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RepositoryRevisionLabel.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
