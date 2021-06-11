CREATE TABLE orginization (
    org_id INTEGER PRIMARY KEY NOT NULL,
    org_name TEXT UNIQUE NOT NULL
);

CREATE TABLE repository(
    repo_id INTEGER PRIMARY KEY NOT NULL,
    org_id INTEGER NOT NULL,
    repo_name TEXT NOT NULL,
    url TEXT,
    UNIQUE(org_id, repo_name)
);

CREATE TABLE repository_labels(
    id INTEGER PRIMARY KEY NOT NULL,
    repo_id INTEGER NOT NULL,
    label_name TEXT NOT NULL,
    label_value TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    UNIQUE(repo_id, label_name)
);

CREATE TABLE repository_revision(
    revision_id INTEGER PRIMARY KEY NOT NULL,
    repo_id INTEGER NOT NULL,
    scm_tag TEXT NOT NULL,
    scm_id TEXT NOT NULL,
    UNIQUE(repo_id, scm_tag)
);

CREATE TABLE repository_revision_labels(
    id INTEGER PRIMARY KEY NOT NULL,
    repo_id INTEGER NOT NULL,
    label_name TEXT NOT NULL,
    label_value TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    UNIQUE(repo_id, label_name)
);