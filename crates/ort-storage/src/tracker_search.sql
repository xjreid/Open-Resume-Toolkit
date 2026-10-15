-- Canonical records own both the metadata projection and pre-normalized search text.
-- Keeping this projection in SQLite covers saves, finish, restore, and direct deletes.
CREATE TABLE tracker_search (
    entry_id TEXT PRIMARY KEY REFERENCES tracker_entries(entry_id) ON DELETE CASCADE,
    profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
    revision INTEGER NOT NULL,
    updated_at TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    has_resume INTEGER NOT NULL,
    has_cover_letter INTEGER NOT NULL,
    answer_count INTEGER NOT NULL,
    status TEXT NOT NULL,
    date_applied TEXT NOT NULL,
    search_text TEXT NOT NULL
) STRICT;
CREATE INDEX tracker_search_profile
    ON tracker_search(profile_id, date_applied DESC, updated_at DESC, entry_id DESC);
CREATE INDEX tracker_search_status
    ON tracker_search(profile_id, status, date_applied DESC, updated_at DESC, entry_id DESC);

CREATE VIEW tracker_projection AS
SELECT entry_id, profile_id, revision, updated_at,
    json_object(
        'company', json_extract(CAST(entry_json AS TEXT), '$.company'),
        'title', json_extract(CAST(entry_json AS TEXT), '$.title'),
        'location', json_extract(CAST(entry_json AS TEXT), '$.location'),
        'dateApplied', json_extract(CAST(entry_json AS TEXT), '$.dateApplied'),
        'status', json_extract(CAST(entry_json AS TEXT), '$.status'),
        'customStatus', json_extract(CAST(entry_json AS TEXT), '$.customStatus'),
        'sourceUrl', json_extract(CAST(entry_json AS TEXT), '$.sourceUrl')
    ) AS metadata_json,
    coalesce(json_type(CAST(entry_json AS TEXT), '$.resume') = 'object', 0) AS has_resume,
    coalesce(json_type(CAST(entry_json AS TEXT), '$.coverLetter') = 'text', 0) AS has_cover_letter,
    coalesce(json_array_length(CAST(entry_json AS TEXT), '$.answers'), 0) AS answer_count,
    coalesce(json_extract(CAST(entry_json AS TEXT), '$.status'), '') AS status,
    coalesce(json_extract(CAST(entry_json AS TEXT), '$.dateApplied'), '') AS date_applied,
    lower(CAST(entry_json AS TEXT)) AS search_text
FROM tracker_entries;

INSERT INTO tracker_search SELECT * FROM tracker_projection;
CREATE TRIGGER tracker_search_insert AFTER INSERT ON tracker_entries BEGIN
    INSERT INTO tracker_search SELECT * FROM tracker_projection WHERE entry_id = new.entry_id;
END;
CREATE TRIGGER tracker_search_update AFTER UPDATE ON tracker_entries BEGIN
    DELETE FROM tracker_search WHERE entry_id = old.entry_id;
    INSERT INTO tracker_search SELECT * FROM tracker_projection WHERE entry_id = new.entry_id;
END;
