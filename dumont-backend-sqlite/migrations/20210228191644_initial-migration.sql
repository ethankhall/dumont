CREATE TABLE orginization (
    org_id INTEGER PRIMARY KEY NOT NULL,
    org_name TEXT UNIQUE NOT NULL
);

CREATE TABLE repository(
    repo_id INTEGER PRIMARY KEY NOT NULL,
    org_id INTEGER NOT NULL,
    repo_name TEXT NOT NULL,
    UNIQUE(org_id, repo_name)
);

CREATE TABLE repository_metadata(
    id INTEGER PRIMARY KEY NOT NULL,
    repo_id INTEGER NOT NULL,
    meta_key TEXT NOT NULL,
    meta_value TEXT NOT NULL,
    UNIQUE(repo_id, meta_key)
);

CREATE TABLE repository_tags(
    id INTEGER PRIMARY KEY NOT NULL,
    repo_id INTEGER NOT NULL,
    tag TEXT NOT NULL,
    UNIQUE(repo_id, tag)
);

CREATE TABLE repository_revision(
    revision_id INTEGER PRIMARY KEY NOT NULL,
    repo_id INTEGER NOT NULL,
    scm_tag TEXT NOT NULL,
    scm_id TEXT NOT NULL,
    UNIQUE(repo_id, scm_tag)
);

CREATE TABLE repository_revision_state(
    revision_state_id INTEGER PRIMARY KEY NOT NULL,
    repo_id INTEGER NOT NULL,
    revision_id TEXT NOT NULL,
    state_name TEXT NOT NULL,
    created_at DATETIME NOT NULL
);