ALTER TABLE workbook_updates
	ADD COLUMN origin     bytea CHECK (length(origin) = 16),
	ADD COLUMN client_seq bigint CHECK (client_seq >= 0);

CREATE TABLE workbook_acks (
	workbook_id uuid NOT NULL REFERENCES workbooks(id) ON DELETE CASCADE,
	origin      bytea NOT NULL CHECK (length(origin) = 16),
	client_seq  bigint NOT NULL CHECK (client_seq >= 0),
	updated_at  timestamptz NOT NULL DEFAULT now(),
	PRIMARY KEY (workbook_id, origin)
);
