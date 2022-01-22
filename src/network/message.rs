use serde::{Serialize, Deserialize};
use crate::crypto::hash::{H256};
use crate::block::Block;
use crate::transaction::*;
use std::collections::HashMap;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Ping(String),
    Pong(String),
    // added
    NewBlockHashes(Vec<H256>),
    GetBlocks(Vec<H256>),
    Blocks(Vec<Block>),

    NewTransactionHashes(Vec<H256>),
    GetTransactions(Vec<H256>),
    Transactions(Vec<SignedTransaction>),

    NewStateHashes(Vec<H256>),
    GetStates(Vec<H256>),
    States(HashMap<H256,TxOutput>),

    DeleteStateHashes(Vec<H256>),
    
}
