CREATE TABLE timers (
    id          UUID 	PRIMARY KEY	    DEFAULT gen_random_uuid(),
    user_id 	INTEGER	NOT NULL UNIQUE	REFERENCES users(id)    ON DELETE CASCADE,
    created_at  BIGINT  NOT NULL        DEFAULT EXTRACT(EPOCH FROM NOW())*1000,
    summary     TEXT,
    details     TEXT,
    location    TEXT,
    template    INTEGER NOT NULL REFERENCES local_events(id)    ON DELETE CASCADE
);

CREATE OR REPLACE FUNCTION check_template_user()
RETURNS TRIGGER AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM local_events
        WHERE id = NEW.template
          AND user_id = NEW.user_id
    ) THEN
        RAISE EXCEPTION 'olmonoko.timer.forbidden-template';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER validate_template_user
BEFORE INSERT OR UPDATE ON timers
FOR EACH ROW
EXECUTE FUNCTION check_template_user();
