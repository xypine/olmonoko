-- remove existing duplicates
DELETE FROM event_occurrences
WHERE id NOT IN (
  SELECT MIN(id)
  FROM event_occurrences
  GROUP BY event_id, starts_at
);
-- create unique constraint
CREATE UNIQUE INDEX event_occurrences_unique ON event_occurrences (event_id, starts_at);
