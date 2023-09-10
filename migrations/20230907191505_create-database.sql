CREATE TABLE mining_sessions(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, 
    public_key_hash TEXT NOT NULL,                
    start_time DATETIME NOT NULL,        
    currently_mining_block INTEGER NOT NULL,
    end_time DATETIME,
    payment_due INTEGER,
    payment_transaction TEXT CHECK(length(payment_transaction) = 64),
    is_definitely_paid BOOLEAN NOT NULL
);

CREATE INDEX idx_address ON mining_sessions(public_key_hash);

CREATE TABLE proof_of_work(
    mining_session_id INTEGER NOT NULL,
    block_number INTEGER NOT NULL,                 
    sha TEXT CHECK(length(sha) = 64) NOT NULL,   
    nonce TEXT CHECK(length(nonce) = 32) NOT NULL,
    created_at DATETIME NOT NULL,
    PRIMARY KEY(sha),
    FOREIGN KEY(mining_session_id) REFERENCES mining_sessions(id)
);

CREATE TABLE datum_submissions(
    transaction_hash TEXT CHECK(length(transaction_hash) = 64) NOT NULL,
    sha TEXT CHECK(length(sha) = 64) NOT NULL,   
    is_definitely_accepted BOOLEAN NOT NULL,
    is_definitely_rejected BOOLEAN NOT NULL,
    created_at DATETIME NOT NULL,
    FOREIGN KEY(sha) REFERENCES proof_of_work(sha)
);