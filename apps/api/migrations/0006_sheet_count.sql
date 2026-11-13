ALTER TABLE workbooks ADD COLUMN sheet_count integer CHECK (sheet_count > 0);
