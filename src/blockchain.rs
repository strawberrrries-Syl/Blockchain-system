use std::collections::HashMap;

use crate::block::Block;
use crate::crypto::merkle::*;
//use super::transaction::Transaction;
use super::block::*;
use crate::crypto::hash::{H256, Hashable};
use crate::transaction::*;
use ring::signature::{self, Ed25519KeyPair, Signature, KeyPair};
use rand::Rng;

#[derive( Eq, PartialEq, Debug, Default, Clone)] // Hash,
pub struct Blockchain {
    pub chain :  HashMap<H256, Block>,
    pub tip : H256,
    pub longest_height : u32,
    pub longest_chain : Vec<H256>,

    pub tx_mempool: HashMap<H256, SignedTransaction>,
    pub ledger_state: HashMap<H256, TxOutput>,
}




impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    /// 函数-生成新链
    pub fn new() -> Self {
        let new_nonce: u32 = 2083236893;
        let diffc: [u8; 32] = [0,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
        let zero :  [u8; 32] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
        // 初始化merkle tree，为了防止panic
        let mut input_data: Vec<H256> = Vec::new();
        input_data.push(From::from(diffc));
        let new_tree = MerkleTree::new(&input_data);
        // content
        let new_content = Content{
            transactions : Vec::new(),
            transaction_detail : HashMap::new(),
            height : 0,
        };
        // header
        let new_header = Header{
            parent_pointer : From::from(zero),
            nonce : new_nonce,
            difficulty : From::from(diffc),
            timestamp : 1231006505,
            merkle_root : new_tree.root(),
        };
        let headerhash = new_header.clone().hash();
        let new_block = Block{
            header :  new_header,
            content : new_content,
        };

        let mut veclongest: Vec<H256> = Vec::new();
        veclongest.push(headerhash);
        let mut map = HashMap::new();
        map.insert(headerhash, new_block);

        // 初始化给4个可花state
        let mut state_init = HashMap::new();
        //let mut rng = rand::thread_rng();
        //for i in 0..4 {
          //  let rand_txhash:[u8;32] = rng.gen();
            //let rd_txhash:H256 = rand_txhash.into();
            //state_init.insert(rd_txhash, generate_state(keypair));
        //}
        
        Blockchain{
            chain: map,
            tip: headerhash,
            longest_height : 0,
            longest_chain : veclongest,

            tx_mempool : HashMap::new(),
            ledger_state: state_init, 
        }
        //unimplemented!()
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        let mut blockcln = block.clone();
        // 确定parent
        let previousblock = block.header.parent_pointer;    // 确定插入块的parent 哈希值
        // 初始化parent block为插入的block
        // 遍历chain中的block，找前一个块，存在parent_block中
        
        //找到父块，确定当前块高度，以及当前块是否是最长链第一个到达的块
        let parentblock = self.chain.get(&previousblock).unwrap();   // 找到父块
        let tipblock = self.chain.get(&self.tip).unwrap();   // 找到最高块

        let txs = block.content.transactions.clone();   //块中包含的tx

        blockcln.content.height = parentblock.content.height + 1;       // 确认插入块高度
        if (parentblock.content.height + 1) == self.longest_height{    // 如果新块的父块高度是当前最高高度 - 1
            if tipblock.header.timestamp > blockcln.header.timestamp {      // 当前块比最高块挖的早
                // 原先height
                let old_tip_block = self.chain.get(&self.tip()).unwrap().clone();
                let old_tx = old_tip_block.content.transactions.clone();
                // 改tip
                self.tip = blockcln.header.hash();
                self.longest_chain.pop();
                self.longest_chain.push(self.tip);
                self.longest_height = blockcln.content.height;
                // 改tx相关
                // 弹出先前confirmed的state，将交易恢复到mempool中
                for i in old_tx {
                    let transac_now = old_tip_block.content.transaction_detail.get(&i).unwrap().clone();
                    if transac_now.tx.input.prev_tx != [0;32].into() {
                        self.ledger_state.remove(&i);   //弹出state
                        self.tx_mempool.insert(i, transac_now); // 恢复mempool
                    }
                }
                // 加入新state
                for j in txs {
                    self.ledger_state.insert(j, block.content.transaction_detail.get(&j).unwrap().clone().tx.output);
                }
            }
        } else if blockcln.content.height > self.longest_height {
            self.tip = blockcln.header.hash();
            self.longest_chain.push(self.tip);
            self.longest_height = blockcln.content.height;
            // 加入新state
            for j in txs {
                let info = block.content.transaction_detail.get(&j).unwrap().clone();
                if info.tx.input.prev_tx != [0;32].into() {
                    self.ledger_state.insert(j, block.content.transaction_detail.get(&j).unwrap().clone().tx.output);
                }
                
            }
        } 

        // 插入块
        self.chain.insert(blockcln.header.hash(), blockcln);
        println!("Longest chain: {:?}", self.longest_chain);
        let mut memp = Vec::new();
        let mut state = Vec::new();

        for i in self.tx_mempool.keys() {
            memp.push(i);
        }

        for j in self.ledger_state.keys() {
            state.push(j);
        }


        println!("tx mempool: {:?}",memp);
        println!("state: {:?}",state);
        // unimplemented!()
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        self.tip
        //unimplemented!()
    }

    /// Get the last block's hash of the longest chain
    #[cfg(any(test, test_utilities))]
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        unimplemented!()
    }
}

#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::block::test::generate_random_block;
    use crate::crypto::hash::Hashable;

    //#[test]
    //fn insert_one() {
        //let mut blockchain = Blockchain::new();
        //let genesis_hash = blockchain.tip();
        //let block = generate_random_block(&genesis_hash);
        //blockchain.insert(&block);
        //assert_eq!(blockchain.tip(), block.hash());

    //}
}
