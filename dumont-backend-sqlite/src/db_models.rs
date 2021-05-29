#[derive(Debug, sqlx::FromRow)]
pub struct OrganizationDbModel {
    pub org_id: i64,
    pub org_name: String,
}
