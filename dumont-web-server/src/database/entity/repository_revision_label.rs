//! SeaORM Entity. Generated by sea-orm-codegen 0.6.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "repository_revision_label")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub revision_label_id: i32,
    pub revision_id: i32,
    #[sea_orm(column_type = "Text")]
    pub label_name: String,
    #[sea_orm(column_type = "Text")]
    pub label_value: String,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::repository_revision::Entity",
        from = "Column::RevisionId",
        to = "super::repository_revision::Column::RevisionId",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    RepositoryRevision,
}

impl Related<super::repository_revision::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RepositoryRevision.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
