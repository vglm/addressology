CREATE TABLE miner_info (
    uid TEXT NOT NULL,
    prov_node_id TEXT NULL,
    prov_reward_addr TEXT NULL,
    prov_name TEXT NULL,
    prov_extra_info TEXT NULL,
    CONSTRAINT miner_info_pk PRIMARY KEY (uid)
) STRICT;

CREATE TABLE job_info (
    uid TEXT NOT NULL,
    cruncher_ver TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT NULL,
    requestor_id TEXT NULL,
    hashes_reported REAL NOT NULL,
    hashes_accepted REAL NOT NULL,
    cost_reported REAL NOT NULL,
    miner TEXT NULL,
    job_extra_info TEXT NULL,
    CONSTRAINT job_info_pk PRIMARY KEY (uid),
    FOREIGN KEY (miner) REFERENCES miner_info (uid)
) STRICT;

ALTER TABLE fancy RENAME TO fancy_old;

CREATE TABLE fancy (
    address TEXT NOT NULL,
    salt TEXT NOT NULL,
    factory TEXT NOT NULL,
    created TEXT NOT NULL,
    score REAL NOT NULL,
    job TEXT NULL,
    owner TEXT NULL,
    price INTEGER NOT NULL DEFAULT 1000,
    category TEXT NULL,
    CONSTRAINT fancy_pk PRIMARY KEY (address),
    CONSTRAINT fancy_fk FOREIGN KEY (job) REFERENCES job_info (uid)
) STRICT;

INSERT INTO fancy (address, salt, factory, created, score, owner, price, category)
SELECT address, salt, factory, created, score, owner, price, category
FROM fancy_old;
DROP TABLE fancy_old;
