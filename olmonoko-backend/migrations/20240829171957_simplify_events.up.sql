CREATE TABLE remote_local_link (
    local_event_id        INTEGER REFERENCES local_events(id) ON DELETE CASCADE NOT NULL,
    remote_occurrence_id    INTEGER REFERENCES event_occurrences(id) ON DELETE CASCADE NOT NULL,
    created_at BIGINT DEFAULT EXTRACT(EPOCH FROM NOW())*1000 NOT NULL
);

-- Step 1: Add attendance-related fields to local_events table
ALTER TABLE local_events
DROP COLUMN rrule,
ADD COLUMN auto_imported BOOLEAN DEFAULT FALSE NOT NULL,
ADD COLUMN attendance_planned BOOLEAN DEFAULT FALSE NOT NULL,
ADD COLUMN attendance_actual BOOLEAN DEFAULT FALSE NOT NULL;

-- Step 2: Create a function to get the first occurrence of a remote event
CREATE OR REPLACE FUNCTION get_first_occurrence(remote_event_id INTEGER) 
RETURNS INTEGER AS $$
DECLARE
    first_occurrence_id INTEGER;
BEGIN
    SELECT id INTO first_occurrence_id
    FROM event_occurrences
    WHERE event_id = remote_event_id
    ORDER BY starts_at ASC
    LIMIT 1;
    
    RETURN first_occurrence_id;
END;
$$ LANGUAGE plpgsql;

-- Step 3: Create new local events and transfer data, including attendance
WITH 
source AS (
    SELECT 
        ics.user_id,
        EXTRACT(EPOCH FROM NOW()) * 1000 as created_at,
        EXTRACT(EPOCH FROM NOW()) * 1000 as updated_at,
        eo.starts_at,
        e.duration,
        e.summary,
        e.description,
        e.location,
        e.uid,
        e.all_day,
        e.priority_override as priority,
        COALESCE(a.planned, FALSE) as attendance_planned,
        COALESCE(a.actual, FALSE) as attendance_actual,
        TRUE as auto_imported,
        eo.id as occurrence_id
    FROM events e
    JOIN event_occurrences eo ON e.id = eo.event_id
    JOIN ics_sources ics ON e.event_source_id = ics.id
    LEFT JOIN attendance a ON e.id = a.remote_event_id AND ics.user_id = a.user_id
    WHERE e.id IN (
        -- SELECT DISTINCT remote_event_id FROM event_tags
        -- UNION
        SELECT DISTINCT remote_event_id FROM bills
        UNION
        SELECT DISTINCT remote_event_id FROM attendance
    )
    AND eo.id = get_first_occurrence(e.id)
),
nl AS (
    INSERT INTO local_events (
        user_id, created_at, updated_at, starts_at, duration, summary, 
        description, location, uid, all_day, priority,
        attendance_planned, attendance_actual, auto_imported
    ) SELECT user_id, created_at, updated_at, starts_at, duration, summary, description, location, uid, all_day, priority, attendance_planned, attendance_actual, auto_imported FROM source
    RETURNING *
)
INSERT INTO remote_local_link (
    local_event_id,
    remote_occurrence_id
) SELECT nl.id, source.occurrence_id FROM source, nl WHERE source.uid = nl.uid;

-- Step 6: Perform schema changes
ALTER TABLE event_tags DROP COLUMN remote_event_id;
DELETE FROM event_tags WHERE local_event_id is NULL;
ALTER TABLE event_tags ALTER COLUMN local_event_id SET NOT NULL;
ALTER TABLE bills DROP COLUMN remote_event_id;
DELETE FROM bills WHERE local_event_id is NULL;
ALTER TABLE bills ALTER COLUMN local_event_id SET NOT NULL;
DROP TABLE IF EXISTS attendance;

-- Step 7: Clean up
DROP FUNCTION get_first_occurrence;
