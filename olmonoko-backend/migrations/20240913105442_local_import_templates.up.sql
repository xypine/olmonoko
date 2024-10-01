ALTER TABLE ics_source_priorities ADD COLUMN import_template 		TEXT;

ALTER TABLE ics_source_priorities ADD COLUMN imported_at		TEXT;
ALTER TABLE ics_source_priorities ADD COLUMN imported_hash		TEXT;
ALTER TABLE ics_source_priorities ADD COLUMN imported_hash_version	TEXT;
