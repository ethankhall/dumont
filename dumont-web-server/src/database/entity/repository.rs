//! SeaORM Entity. Generated by sea-orm-codegen 0.6.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "repository")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub repo_id: i32,
    pub org_id: i32,
    #[sea_orm(column_type = "Text")]
    pub repo_name: String,
    pub created_at: DateTimeUtc,
    #[sea_orm(column_type = "Text", nullable)]
    pub url: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::organization::Entity",
        from = "Column::OrgId",
        to = "super::organization::Column::OrgId",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Organization,
    #[sea_orm(has_many = "super::repository_label::Entity")]
    RepositoryLabel,
    #[sea_orm(has_many = "super::repository_revision::Entity")]
    RepositoryRevision,
}

impl Related<super::organization::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organization.def()
    }
}

impl Related<super::repository_label::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RepositoryLabel.def()
    }
}

impl Related<super::repository_revision::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RepositoryRevision.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
