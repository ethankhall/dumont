use dumont_models::models::{Organization, Repository};

#[derive(Debug, sqlx::FromRow)]
pub struct OrganizationDbModel {
    pub org_id: i64,
    pub org_name: String,
}

impl OrganizationDbModel {
    pub(crate) fn into_org(org: &OrganizationDbModel) -> Organization {
        Organization {
            id: org.org_id,
            name: org.org_name.clone(),
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct OrganizationIdDbModel {
    pub org_id: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct RepoDbModel {
    pub org_id: i64,
    pub repo_id: i64,
    pub repo_name: String,
    pub url: Option<String>,
}

impl RepoDbModel {
    pub(crate) fn into_repo(org: &OrganizationDbModel, repo: &RepoDbModel) -> Repository {
        Repository {
            id: repo.repo_id,
            name: repo.repo_name.clone(),
            url: repo.url.clone(),
            organization: OrganizationDbModel::into_org(org),
        }
    }
}
