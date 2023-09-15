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
    PRIMARY KEY(sha, block_number),
    FOREIGN KEY(miner_id) REFERENCES miners(id)
);

CREATE TABLE datum_submissions(
    transaction_hash TEXT CHECK(length(transaction_hash) = 64) NOT NULL,
    sha TEXT CHECK(length(sha) = 64) NOT NULL,
    block_number INTEGER NOT NULL,
    created_at DATETIME NOT NULL,
    confirmed_in_slot INTEGER,
    confirmed_at DATETIME,
    rejected BOOLEAN NOT NULL,
    PRIMARY KEY (transaction_hash),
    FOREIGN KEY(sha, block_number) REFERENCES proof_of_work(sha, block_number)
);

CREATE INDEX idx_miner_id ON proof_of_work(miner_id);
CREATE INDEX idx_pow_created_at ON proof_of_work(created_at);
CREATE INDEX datum_submission_created_at ON datum_submissions(created_at);
CREATE INDEX datum_submission_confirmed_at ON datum_submissions(confirmed_at);