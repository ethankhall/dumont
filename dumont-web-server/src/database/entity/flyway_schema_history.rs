//! SeaORM Entity. Generated by sea-orm-codegen 0.6.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "flyway_schema_history")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub installed_rank: i32,
    pub version: Option<String>,
    pub description: String,
    pub r#type: String,
    pub script: String,
    pub checksum: Option<i32>,
    pub installed_by: String,
    pub installed_on: DateTimeUtc,
    pub execution_time: i32,
    pub success: bool,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No RelationDef")
    }
}

impl ActiveModelBehavior for ActiveModel {}
