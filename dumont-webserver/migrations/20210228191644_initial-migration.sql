CREATE TABLE organization (
    org_id INTEGER PRIMARY KEY NOT NULL,
    org_name TEXT UNIQUE NOT NULL
);

CREATE TABLE repository(
    repo_id INTEGER PRIMARY KEY NOT NULL,
    org_id INTEGER NOT NULL REFERENCES organization(org_id),
    repo_name TEXT NOT NULL,
    url TEXT,
    UNIQUE(org_id, repo_name)
);

CREATE TABLE repository_metadata(
    repository_metadata_id INTEGER PRIMARY KEY NOT NULL,
    repo_id INTEGER NOT NULL REFERENCES repository(repo_id),
    repo_url TEXT,
    UNIQUE(repo_id)
);

CREATE TABLE repository_label(
    repository_label_id INTEGER PRIMARY KEY NOT NULL,
    repo_id INTEGER NOT NULL REFERENCES repository(repo_id),
    label_name TEXT NOT NULL,
    label_value TEXT NOT NULL,
    created_at timestamp NOT NULL,
    UNIQUE(repo_id, label_name)
);

CREATE TABLE repository_revision(
    revision_id INTEGER PRIMARY KEY NOT NULL,
    repo_id INTEGER NOT NULL REFERENCES repository(repo_id),
    revision_name TEXT NOT NULL,
    scm_id TEXT NOT NULL,
    created_at timestamp NOT NULL,
    artifact_url TEXT,
    UNIQUE(repo_id, revision_name)
);

CREATE TABLE repository_revision_label(
    revision_label_id INTEGER PRIMARY KEY NOT NULL,
    revision_id INTEGER NOT NULL REFERENCES repository_revision(revision_id),
    label_name TEXT NOT NULL,
    label_value TEXT NOT NULL,
    created_at timestamp NOT NULL,
    UNIQUE(revision_id, label_name)
);
