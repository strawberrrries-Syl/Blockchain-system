use serde::{Serialize, Deserialize};
use rand::Rng;
use std::collections::HashMap;
use chrono::prelude::*;

use crate::crypto::hash::{H256, Hashable};
use crate::crypto::merkle::*;
use super::transaction::*;

#[derive(Hash, Eq, PartialEq,Debug, Default, Clone, Serialize, Deserialize)]
pub struct Header {
    pub parent_pointer: H256,
    pub nonce: u32,
    pub difficulty: H256,
    pub timestamp: i64,
    pub merkle_root: H256,
}


#[derive(Eq, PartialEq,Debug, Default,Clone, Serialize, Deserialize)]
pub struct Content {
    pub transactions: Vec<H256>,
    pub transaction_detail : HashMap<H256,SignedTransaction>,
    pub height : u32,
}

// block contains head and content of transactions
#[derive(Eq, PartialEq,Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub content: Content,
}



impl Hashable for Header {
    fn hash(&self) -> H256 {
        let strtype = serde_json::to_string(&self).unwrap();
        let bytetype = strtype.into_bytes();
        ring::digest::digest(&ring::digest::SHA256, &bytetype).into()
        //unimplemented!()
    }
}

impl Hashable for Transaction {
    fn hash(&self) -> H256 {
        let strtype = serde_json::to_string(&self).unwrap();
        let bytetype = strtype.into_bytes();
        ring::digest::digest(&ring::digest::SHA256, &bytetype).into()

        //unimplemented!()
    }
}

impl Hashable for SignedTransaction {
    fn hash(&self) -> H256 {
        let strtype = serde_json::to_string(&self).unwrap();
        let bytetype = strtype.into_bytes();
        ring::digest::digest(&ring::digest::SHA256, &bytetype).into()
        //unimplemented!()
    }
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
            self.header.hash()
        //unimplemented!()
    }
}


#[cfg(any(test, test_utilities))]
pub mod test {
    use super::*;
    use crate::crypto::hash::H256;

    pub fn generate_random_block(parent: &H256) -> Block {
        let mut rng = rand::thread_rng();
        let new_nonce: u32 = rng.gen();     // nonce generated
        let diffc: [u8; 32] = [0;32];
        let mut input_data: Vec<H256> = Vec::new();
        input_data.push(From::from(diffc));

        let new_tree = MerkleTree::new(&input_data);
        let new_content = Content{
            transactions : Vec::new(),
            transaction_detail :HashMap::new(),
            height : 0,
        };
        let new_header = Header{
            parent_pointer : *parent,
            nonce : new_nonce,
            difficulty : From::from(diffc),
            timestamp : Local::now().timestamp_millis(),
            merkle_root : new_tree.root(),
        };
        Block{
            header :  new_header,
            content : new_content,
        }
        // unimplemented!()
    }
}
