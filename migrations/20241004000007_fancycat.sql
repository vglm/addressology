ALTER TABLE fancy ADD COLUMN category TEXT NULL;

CREATE INDEX fancy_category_idx ON fancy (category);
