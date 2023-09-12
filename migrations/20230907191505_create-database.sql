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
    created_at DATETIME NOT NULL,
    confirmed_in_slot INTEGER,
    confirmed_at DATETIME,
    rejected BOOLEAN NOT NULL,
    PRIMARY KEY (transaction_hash),
    FOREIGN KEY(sha) REFERENCES proof_of_work(sha)
);

CREATE TABLE payouts(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    datum_transaction_hash TEXT CHECK(length(datum_transaction_hash) = 64) NOT NULL,
    miner_id INTEGER NOT NULL,
    paid_amount INTEGER NOT NULL,
    created_at DATETIME NOT NULL,
    FOREIGN KEY(miner_id) REFERENCES miners(id),
    FOREIGN KEY(datum_transaction_hash) REFERENCES datum_submissions(transaction_hash)
);