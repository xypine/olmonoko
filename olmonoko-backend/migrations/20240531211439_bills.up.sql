CREATE TABLE bills (
    id SERIAL PRIMARY KEY NOT NULL,
    -- either local_event_id or remote_event_id must be set
    -- but not both
    local_event_id INTEGER REFERENCES local_events(id) ON DELETE CASCADE,
    remote_event_id INTEGER REFERENCES events(id) ON DELETE CASCADE,

    -- fields present in https://www.finanssiala.fi/wp-content/uploads/2021/03/Bank_bar_code_guide.pdf
    payee_account_number TEXT NOT NULL,
    amount INTEGER NOT NULL, -- in € cents
    reference TEXT NOT NULL,
    due_at INTEGER NOT NULL,

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

CREATE TABLE bill_payments (
    id SERIAL PRIMARY KEY NOT NULL,
    bill_id INTEGER REFERENCES bills(id) ON DELETE CASCADE,
    -- either local_event_id or remote_event_id must be set
    -- but not both
    local_event_id INTEGER REFERENCES local_events(id) ON DELETE CASCADE,
    remote_event_id INTEGER REFERENCES events(id) ON DELETE CASCADE,


    paid_at INTEGER NOT NULL,
    amount INTEGER NOT NULL, -- in € cents
    reference TEXT NOT NULL,
    created_at INTEGER DEFAULT (strftime('%s', 'now')) NOT NULL
);
