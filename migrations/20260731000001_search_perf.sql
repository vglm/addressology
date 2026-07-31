CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE INDEX IF NOT EXISTS fancy_created_idx ON fancy (created DESC);
CREATE INDEX IF NOT EXISTS fancy_category_score_idx ON fancy (category, score DESC) INCLUDE (owner_id);
CREATE INDEX IF NOT EXISTS fancy_category_created_idx ON fancy (category, created DESC) INCLUDE (owner_id);
CREATE INDEX IF NOT EXISTS fancy_address_prefix_idx ON fancy (address varchar_pattern_ops);
CREATE INDEX IF NOT EXISTS fancy_address_trgm_idx ON fancy USING gin (address gin_trgm_ops);
