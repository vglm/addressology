



CREATE TABLE miner_info (
    id INT NOT NULL,
    cruncher_ver TEXT NOT NULL,
    mined_by TEXT NOT NULL,
    provider_id TEXT NULL,
    provider_name TEXT NULL,
    extra_info TEXT NULL,
    CONSTRAINT miner_info_pk PRIMARY KEY (id)
) strict;

CREATE TABLE job_info (
    id INT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT NOT NULL,
    requestor_id TEXT NULL,
    hashes_computed REAL NOT NULL,
    glm_spent REAL NOT NULL,
    miner_id INT NULL,
    job_extra_info TEXT NULL,
    CONSTRAINT job_info_pk PRIMARY KEY (id),
    FOREIGN KEY (miner_id) REFERENCES miner_info (id)
) strict;

ALTER TABLE fancy RENAME TO fancy_old;
CREATE TABLE fancy
(
    address             TEXT NOT NULL,
    salt                TEXT NOT NULL,
    factory             TEXT NOT NULL,
    created             TEXT NOT NULL,
    score               REAL NOT NULL,
    miner_info          INT NULL,
    owner               TEXT NULL,
    price               INT NOT NULL DEFAULT 1000,
    category            TEXT NULL,

    CONSTRAINT fancy_pk PRIMARY KEY (address),
    CONSTRAINT fancy_fk FOREIGN KEY (miner_info) REFERENCES miner_info (id)
) strict;

INSERT INTO fancy (address, salt, factory, created, score, owner, price, category)
SELECT address, salt, factory, created, score, owner, price, category
FROM fancy_old;

CREATE INDEX miner_total_idx ON miner_info (cruncher_ver, mined_by, provider_id, provider_name, extra_info);