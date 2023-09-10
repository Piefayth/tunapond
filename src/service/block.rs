use std::collections::{VecDeque, HashMap};
use std::sync::{Arc, RwLock};
use std::env;
use cardano_multiplatform_lib::plutus::{PlutusData};
use serde::{Deserialize, Serialize};

const MAX_ITEMS: usize = 10;  // For example
const TUNA_CONTRACT_NFT_POLICY_MAINNET: &str = "279f842c33eed9054b9e3c70cd6a3b32298259c24b78b895cb41d91a.6c6f72642074756e61";
const TUNA_CONTRACT_NFT_POLICY_PREVIEW: &str = "502fbfbdafc7ddada9c335bd1440781e5445d08bada77dc2032866a6.6c6f72642074756e61";

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ReadableBlock {
    pub block_number: i64,
    pub current_hash: String,
    pub leading_zeroes: u8,
    pub difficulty_number: u16,
    pub epoch_time: u64,
    pub current_time: u64,
    pub extra: String,
    pub interlink: Vec<String>,
}

impl From<Block> for ReadableBlock {
    fn from(block: Block) -> Self {
        ReadableBlock {
            block_number: block.block_number,
            current_hash: hex::encode(block.current_hash),
            leading_zeroes: block.leading_zeroes,
            difficulty_number: block.difficulty_number,
            epoch_time: block.epoch_time,
            current_time: block.current_time,
            extra: hex::encode(block.extra),
            interlink: block.interlink.into_iter().map(|x| hex::encode(x)).collect(),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq, Hash)]
pub struct Block {
    pub block_number: i64,
    pub current_hash: Vec<u8>,
    pub leading_zeroes: u8,
    pub difficulty_number: u16,
    pub epoch_time: u64,
    pub current_time: u64,
    pub extra: Vec<u8>,
    pub interlink: Vec<Vec<u8>>,
    pub output_index: i64,
    pub transaction_id: String,
}


#[derive(Debug, Deserialize)]
struct KupoDatumResponse {
    datum: String
}

#[derive(Debug, Deserialize)]
struct KupoTransaction {
    datum_hash: String,
    value: KupoValue,
    output_index: i64,
    transaction_id: String,
}

#[derive(Debug, Deserialize)]
struct KupoValue {
    coins: u64,
    assets: HashMap<String, u64>,
}

#[derive(Debug)]
pub enum BlockServiceError {
    ReqwestError(reqwest::Error),
    LockError,
    NoMatchingContractTransaction,
    BlockParseFailure
}

impl From<reqwest::Error> for BlockServiceError {
    fn from(err: reqwest::Error) -> Self {
        BlockServiceError::ReqwestError(err)
    }
}

pub struct BlockService {
    history: Arc<RwLock<VecDeque<Block>>>,
    kupo_url: String,
    contract_address: String
}

impl BlockService {
    pub fn new() -> Self {
        let history = Arc::new(RwLock::new(VecDeque::with_capacity(MAX_ITEMS)));
        let kupo_url = env::var("KUPO_URL").expect("Cannot instantiate BlockService because KUPO_URL is not set.");
        let contract_address = env::var("CONTRACT_ADDRESS")
            .unwrap_or(String::from("addr1wynelppvx0hdjp2tnc78pnt28veznqjecf9h3wy4edqajxsg7hwsc"));

        BlockService { 
            history,
            kupo_url,
            contract_address
        }
    }

    pub fn get_latest(&self) -> Result<Block, BlockServiceError> {
        let default_block = Block::default();
        let read_history = self.history.read().map_err(|_| {
            log::warn!("Could not acquire read access to block service history. History was not updated.");
            BlockServiceError::LockError
        })?;
        Ok(read_history.front().unwrap_or(&default_block).clone()) // We clone to own the data outside the lock
    }

    async fn update_history(&self) -> Result<(), BlockServiceError> {
        let network = std::env::var("NETWORK").unwrap_or(String::from("Mainnet"));
        let nft_policy = match &*network {
            "Preview" => TUNA_CONTRACT_NFT_POLICY_PREVIEW,
            _ => TUNA_CONTRACT_NFT_POLICY_MAINNET
        };

        let all_contract_unspent_tx: Vec<KupoTransaction> = reqwest::get(
                format!("{}/matches/{}?unspent", self.kupo_url, self.contract_address)
            )
            .await?
            .json()
            .await?;
        
        let most_recent_datum_tx: KupoTransaction = all_contract_unspent_tx.into_iter()
            .find(|tx| {
                tx.value.assets.get(nft_policy) == Some(&1)
            })
            .ok_or(BlockServiceError::NoMatchingContractTransaction)?;

        
        let most_recent_datum: KupoDatumResponse = reqwest::get(format!("{}/datums/{}", self.kupo_url, most_recent_datum_tx.datum_hash))
            .await?
            .json()
            .await?;
        
        let default_block = Block::default();
        let most_recent_block = block_from_datum(most_recent_datum.datum, most_recent_datum_tx)?;

        let last_seen_block = {
            let read_history = self.history.read().map_err(|_| {
                log::warn!("Could not acquire read access to block service history. History was not updated.");
                BlockServiceError::LockError
            })?;
            read_history.front().unwrap_or(&default_block).clone() // We clone to own the data outside the lock
        };

        if most_recent_block.block_number > last_seen_block.block_number {
            let mut write_history = self.history.write().map_err(|_| {
                log::warn!("Could not acquire write access to block service history. History was not updated.");
                BlockServiceError::LockError
            })?;

            if write_history.len() == MAX_ITEMS {
                write_history.pop_back();
            }
            
            log::info!("Fetched new block {} from upstream and updated BlockService history.", &most_recent_block.block_number);
            write_history.push_front(most_recent_block);
        } else {
            log::debug!("Successfully fetched from upstream, but no updates for BlockService found.");
        }

        Ok(())
    }
}

pub async fn block_updater(service: Arc<BlockService>) {
    let default_interval = 20;
    let datum_update_interval: u64 = std::env::var("DATUM_UPDATE_INTERVAL")
        .unwrap_or_else(|_| default_interval.to_string())
        .parse()
        .unwrap_or(default_interval);

    loop {
        let update = service.update_history().await;
        match update {
            Ok(_) => {
                // good
            },
            Err(err) => {
                println!("Block updater error: |{:?}|", err);
            },
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(datum_update_interval)).await;
    }
}


fn block_from_datum(datum: String, tx: KupoTransaction) -> Result<Block, BlockServiceError> {
    let hex_bytes = hex::decode(datum.as_bytes())
        .map_err(|_| {
            log::warn!("Could not decode hex from datum {}.", datum);
            BlockServiceError::BlockParseFailure
        })?;

    let data = PlutusData::from_bytes(hex_bytes)
        .map_err(|_| {
            log::warn!("Could not create plutus data for block from datum bytes. {}.", datum);
            BlockServiceError::BlockParseFailure
        })?;

    let typed_data = data
        .as_constr_plutus_data()
        .ok_or_else(|| {
            log::warn!("Could not create constr plutus data for block from parsed plutus data. {}.", datum);
            BlockServiceError::BlockParseFailure
        })?;
    
    let block_number = typed_data.data().get(0).as_integer().unwrap().as_int().unwrap().as_i32_or_nothing().unwrap() as i64;
    let current_hash = typed_data.data().get(1).as_bytes().unwrap();
    let leading_zeroes = typed_data.data().get(2).as_integer().unwrap().as_int().unwrap().as_i32_or_nothing().unwrap() as u8;
    let difficulty_number = typed_data.data().get(3).as_integer().unwrap().as_int().unwrap().as_i32_or_nothing().unwrap() as u16;
    let epoch_time = typed_data.data().get(4).as_integer().unwrap().as_u64().unwrap();
    let current_time = typed_data.data().get(5).as_integer().unwrap().as_u64().unwrap();
    let extra = typed_data.data().get(6).to_bytes();    // TODO: Handle this some other way? It could be many types
    let interlink_list = typed_data.data().get(7).as_list().unwrap();
    let mut interlink: Vec<Vec<u8>> = Vec::new();
    
    for i in 0..interlink_list.len() {
        if let Some(bytes) = interlink_list.get(i).as_bytes() {
            interlink.push(bytes.to_vec());
        }
    }
    

    let block = Block {
        block_number,
        current_hash,
        leading_zeroes: leading_zeroes,
        difficulty_number,
        epoch_time: epoch_time.into(),  
        current_time: current_time.into(), 
        extra,
        interlink,
        transaction_id: tx.transaction_id,
        output_index: tx.output_index
    };

    Ok(block)
}