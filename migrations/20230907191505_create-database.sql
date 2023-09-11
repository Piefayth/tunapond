CREATE TABLE miners(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    address TEXT NOT NULL,
    pkh TEXT NOT NULL
);

CREATE TABLE proof_of_work(
    miner_id INTEGER NOT NULL,
    block_number INTEGER NOT NULL,                 
    sha TEXT CHECK(length(sha) = 64) NOT NULL,   
    nonce TEXT CHECK(length(nonce) = 32) NOT NULL,
    created_at DATETIME NOT NULL,
    PRIMARY KEY(sha),
    FOREIGN KEY(miner_id) REFERENCES miners(id)
);

CREATE INDEX idx_miner_id ON proof_of_work(miner_id);

CREATE TABLE datum_submissions(
    transaction_hash TEXT CHECK(length(transaction_hash) = 64) NOT NULL,
    sha TEXT CHECK(length(sha) = 64) NOT NULL,   
    is_definitely_accepted BOOLEAN NOT NULL,
    is_definitely_rejected BOOLEAN NOT NULL,
    created_at DATETIME NOT NULL,
    paid_at DATETIME,
    FOREIGN KEY(sha) REFERENCES proof_of_work(sha)
);

CREATE TABLE payouts_due(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    miner_id INTEGER NOT NULL,
    owed INTEGER NOT NULL,
    is_paid BOOLEAN NOT NULL,
    created_at DATETIME NOT NULL,
    transaction_hash TEXT CHECK(length(transaction_hash) = 64),
    transaction_time DATETIME,
    FOREIGN KEY(miner_id) REFERENCES miners(id)
);