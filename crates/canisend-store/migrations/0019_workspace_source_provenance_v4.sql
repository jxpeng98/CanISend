ALTER TABLE workspace_source_v4_revisions
    ADD COLUMN final_locator TEXT;

ALTER TABLE workspace_source_v4_revisions
    ADD COLUMN redirect_chain_json TEXT NOT NULL DEFAULT '[]';

PRAGMA user_version = 19;
