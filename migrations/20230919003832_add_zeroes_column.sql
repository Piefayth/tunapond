ALTER TABLE proof_of_work 
-- specifically, the miner's elected sampling difficulty at the time the pow was submitted
ADD COLUMN sampling_difficulty INTEGER NOT NULL DEFAULT 8;  

ALTER TABLE miners
ADD COLUMN sampling_difficulty INTEGER NOT NULL DEFAULT 8;