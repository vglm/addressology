CREATE TABLE miner_info (
                            id INTEGER NOT NULL,
                            cruncher_ver TEXT NOT NULL,
                            mined_by TEXT NOT NULL,
                            provider_id TEXT NULL,
                            provider_name TEXT NULL,
                            extra_info TEXT NULL,
                            CONSTRAINT miner_info_pk PRIMARY KEY (id)
) STRICT;

CREATE TABLE job_info (
                          id INTEGER NOT NULL,
                          started_at DATETIME NOT NULL,
                          finished_at DATETIME NOT NULL,
                          requestor_id TEXT NULL,
                          hashes_computed REAL NOT NULL,
                          glm_spent REAL NOT NULL,
                          miner_id INTEGER NULL,
                          job_extra_info TEXT NULL,
                          CONSTRAINT job_info_pk PRIMARY KEY (id),
                          FOREIGN KEY (miner_id) REFERENCES miner_info (id)
) STRICT;

ALTER TABLE fancy RENAME TO fancy_old;

CREATE TABLE fancy (
                       address TEXT NOT NULL,
                       salt TEXT NOT NULL,
                       factory TEXT NOT NULL,
                       created DATETIME NOT NULL,
                       score REAL NOT NULL,
                       miner_info INTEGER NULL,
                       owner TEXT NULL,
                       price INTEGER NOT NULL DEFAULT 1000,
                       category TEXT NULL,
                       CONSTRAINT fancy_pk PRIMARY KEY (address),
                       CONSTRAINT fancy_fk FOREIGN KEY (miner_info) REFERENCES miner_info (id)
) STRICT;

INSERT INTO fancy (address, salt, factory, created, score, owner, price, category)
SELECT address, salt, factory, created, score, owner, price, category
FROM fancy_old;

-- Consider creating indexes based on query patterns
CREATE INDEX miner_cruncher_ver_idx ON miner_info (cruncher_ver);
CREATE INDEX miner_mined_by_idx ON miner_info (mined_by);
CREATE INDEX miner_provider_id_idx ON miner_info (provider_id);
CREATE INDEX miner_provider_name_idx ON miner_info (provider_name);

CREATE INDEX miner_total_idx ON miner_info (cruncher_ver, mined_by, provider_id, provider_name, extra_info);

-- Drop the old table if no longer needed
DROP TABLE fancy_old;