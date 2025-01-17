ALTER TABLE fancy ADD COLUMN owner TEXT NULL;
ALTER TABLE fancy ADD COLUMN price INTEGER NOT NULL DEFAULT 1000;

CREATE INDEX fancy_owner_idx ON fancy (owner);
CREATE INDEX fancy_price_idx ON fancy (price);
CREATE INDEX fancy_score_idx ON fancy (score);

ALTER TABLE users ADD COLUMN tokens INTEGER NOT NULL DEFAULT 1000000;