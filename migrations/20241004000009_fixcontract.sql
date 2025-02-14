DROP TABLE contract;
CREATE TABLE contract
(
    contract_id TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    created     TEXT NOT NULL,
    address     TEXT NULL,
    network     TEXT NOT NULL,
    data        TEXT NOT NULL,
    tx          TEXT NULL,
    deployed    TEXT NULL,
    deploy_status TEXT NOT NULL DEFAULT '',
    deploy_requested TEXT NULL,
    deploy_sent TEXT NULL,

    FOREIGN KEY (user_id) REFERENCES users (uid),
    FOREIGN KEY (address) REFERENCES fancy (address),
    CONSTRAINT contract_pk PRIMARY KEY (contract_id)
) strict;