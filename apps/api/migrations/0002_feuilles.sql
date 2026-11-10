CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE spaces (
	id         text PRIMARY KEY,
	name       text NOT NULL,
	updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE space_members (
	space_id text NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
	subject  text NOT NULL,
	email    text NOT NULL,
	name     text NOT NULL DEFAULT '',
	picture  text NOT NULL DEFAULT '',
	role     text NOT NULL CHECK (role IN ('member', 'admin', 'owner')),
	PRIMARY KEY (space_id, subject)
);
CREATE INDEX space_members_subject_idx ON space_members (subject);

CREATE TABLE directory_state (
	id      smallint PRIMARY KEY CHECK (id = 1),
	version bigint NOT NULL
);

CREATE TABLE deletion_state (
	id         smallint PRIMARY KEY CHECK (id = 1),
	state      bytea NOT NULL,
	updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE workbooks (
	id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
	space_id       text REFERENCES spaces(id) ON DELETE CASCADE,
	owner_id       bigint REFERENCES porte_users(id) ON DELETE SET NULL,
	title          text NOT NULL,
	locale         text NOT NULL DEFAULT 'en',
	timezone       text NOT NULL DEFAULT 'UTC',
	language       text NOT NULL DEFAULT 'en',
	engine_version text NOT NULL,
	snapshot       bytea NOT NULL,
	snapshot_seq   bigint NOT NULL DEFAULT 0,
	seq            bigint NOT NULL DEFAULT 0,
	created_at     timestamptz NOT NULL DEFAULT now(),
	updated_at     timestamptz NOT NULL DEFAULT now(),
	trashed_at     timestamptz,
	CHECK (space_id IS NOT NULL OR owner_id IS NOT NULL)
);
CREATE INDEX workbooks_space_idx ON workbooks (space_id, updated_at DESC) WHERE trashed_at IS NULL;
CREATE INDEX workbooks_owner_idx ON workbooks (owner_id, updated_at DESC) WHERE space_id IS NULL AND trashed_at IS NULL;

CREATE TABLE workbook_updates (
	workbook_id uuid NOT NULL REFERENCES workbooks(id) ON DELETE CASCADE,
	seq         bigint NOT NULL,
	author_id   bigint REFERENCES porte_users(id) ON DELETE SET NULL,
	payload     bytea NOT NULL,
	created_at  timestamptz NOT NULL DEFAULT now(),
	PRIMARY KEY (workbook_id, seq)
);
