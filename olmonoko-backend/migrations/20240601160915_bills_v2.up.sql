-- drop old tables
DROP TABLE IF EXISTS bills;
DROP TABLE IF EXISTS bill_payments;
-- revised version
CREATE TABLE bills (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    -- either local_event_id or remote_event_id must be set
    -- but not both
    local_event_id INTEGER REFERENCES local_events(id) ON DELETE CASCADE,
    remote_event_id INTEGER REFERENCES events(id) ON DELETE CASCADE,

    -- fields present in https://www.finanssiala.fi/wp-content/uploads/2021/03/Bank_bar_code_guide.pdf
    payee_account_number TEXT NOT NULL,
    amount INTEGER NOT NULL, -- in € cents
    reference TEXT NOT NULL,

    -- instead recorded in event starts_at
    -- due_at INTEGER NOT NULL, 
    -- instead recorded in event duration (end - start)
    -- paid_at INTEGER NOT NULL,

    -- additional info
    payee_name TEXT,
    payee_email TEXT,
    payee_address TEXT,
    payee_phone TEXT,

    -- metadata
    created_at INTEGER DEFAULT (strftime('%s', 'now')) NOT NULL,
    updated_at INTEGER DEFAULT (strftime('%s', 'now')) NOT NULL,

    -- either local_event_id or remote_event_id must be set
    CHECK ((local_event_id IS NOT NULL and remote_event_id IS NULL) OR (remote_event_id IS NOT NULL and local_event_id IS NULL))
);
