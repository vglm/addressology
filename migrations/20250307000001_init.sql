CREATE TABLE users (
    uid                 UUID NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
    email               TEXT NOT NULL UNIQUE,
    pass_hash           TEXT,
    created_date        TIMESTAMP NOT NULL DEFAULT now(),
    last_pass_change    TIMESTAMP NOT NULL,
    set_pass_token      TEXT,
    set_pass_token_date TIMESTAMP,
    allow_pass_login    BOOLEAN,
    allow_google_login  BOOLEAN,
    tokens              BIGINT NOT NULL DEFAULT 1000000
);

CREATE TABLE oauth_stage (
    csrf_state TEXT NOT NULL PRIMARY KEY,
    pkce_code_verifier TEXT NOT NULL,
    created_at TIMESTAMP
);

CREATE INDEX oauth_stage_created_at_idx ON oauth_stage (created_at);

CREATE TABLE miner_info (
    uid VARCHAR(64) NOT NULL PRIMARY KEY,
    prov_node_id TEXT NULL,
    prov_reward_addr TEXT NULL,
    prov_name TEXT NULL,
    prov_extra_info TEXT NULL
);

CREATE TABLE job_info (
    uid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cruncher_ver TEXT NOT NULL,
    started_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now(),
    finished_at TIMESTAMP NULL,
    requestor_id TEXT NULL,
    hashes_reported DOUBLE PRECISION NOT NULL,
    hashes_accepted DOUBLE PRECISION NOT NULL,
    entries_accepted BIGINT NOT NULL,
    entries_rejected BIGINT NOT NULL,
    cost_reported DOUBLE PRECISION NOT NULL,
    miner VARCHAR(64) NULL,
    job_extra_info TEXT NULL,
    FOREIGN KEY (miner) REFERENCES miner_info (uid) ON DELETE CASCADE
);

CREATE INDEX job_info_started_at_idx ON job_info (started_at);
CREATE INDEX job_info_updated_at_idx ON job_info (updated_at);
CREATE INDEX job_info_finished_at_idx ON job_info (finished_at);

CREATE TABLE contract_factory (
    uid UUID NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
    address TEXT UNIQUE NOT NULL,
    added TEXT NOT NULL,
    user_id UUID NULL,
    FOREIGN KEY (user_id) REFERENCES users (uid)
);

CREATE TABLE public_key_base (
    uid UUID NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
    hex TEXT UNIQUE NOT NULL,
    added TIMESTAMP NOT NULL,
    user_id UUID NULL,
    FOREIGN KEY (user_id) REFERENCES users (uid)
);

CREATE TABLE fancy (
    address                 VARCHAR(42) NOT NULL PRIMARY KEY,
    salt                    TEXT NOT NULL,
    factory                 TEXT NULL,
    public_key_base         TEXT NULL,
    created                 TIMESTAMP NOT NULL DEFAULT now(),
    score                   DOUBLE PRECISION NOT NULL,
    job_id                  UUID NULL,
    owner_id                UUID NULL,
    price                   BIGINT NOT NULL DEFAULT 1000,
    category                TEXT NULL,
    CONSTRAINT fancy_fk FOREIGN KEY (job_id) REFERENCES job_info (uid),
    CONSTRAINT fancy_fk1 FOREIGN KEY (owner_id) REFERENCES users (uid),
    CONSTRAINT fancy_fk2 FOREIGN KEY (public_key_base) REFERENCES public_key_base (hex) ON DELETE CASCADE,
    CONSTRAINT fancy_fk3 FOREIGN KEY (factory) REFERENCES contract_factory (address) ON DELETE CASCADE
);


CREATE TABLE contract
(
    contract_id         UUID NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL,
    created             TIMESTAMP NOT NULL DEFAULT now(),
    address             TEXT NULL,
    network             TEXT NOT NULL,
    data                TEXT NOT NULL,
    tx                  TEXT NULL,
    deployed            TEXT NULL,
    deploy_status       TEXT NOT NULL DEFAULT '',
    deploy_requested    TIMESTAMP NULL,
    deploy_sent         TIMESTAMP NULL,

    FOREIGN KEY (user_id) REFERENCES users (uid),
    FOREIGN KEY (address) REFERENCES fancy (address)
);

CREATE UNIQUE INDEX contract_address_network_idx ON contract (address, network);

