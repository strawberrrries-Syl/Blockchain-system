use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crossbeam::channel;
use log::{debug, warn};

use std::thread;

use crate::blockchain::*;
use crate::block::*;
use crate::crypto::hash::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;
use crate::transaction::*;
use crate::crypto::key_pair;
use ring::signature::{self, Ed25519KeyPair, Signature, KeyPair};
use chrono::prelude::*;

#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    block_chain: Arc<Mutex<Blockchain>>,        // added for blockchain's tip
    orphan_chain: Arc<Mutex<HashMap<H256,Block>>>,   
}

pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
    orphanchain: &Arc<Mutex<HashMap<H256, Block>>>,     //<parent_pointer, block>
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        block_chain: Arc::clone(blockchain),
        orphan_chain: Arc::clone(orphanchain),
    }
}

impl Context {
    pub fn start(self) {
        println!("worker start");
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&self) {
        loop {
            
            let msg = self.msg_chan.recv().unwrap();
            
            let (msg, peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            // TODO: actual mining
            // 取区块链
            let chain = Arc::clone(&self.block_chain);
            
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
                Message::NewBlockHashes(hashes) => {
                    //debug!("NewBlockHashes");
                    // 接收到消息后，检查hashes列表，找出不在自己链中的hash值，发送get请求
                    let parentchain = chain.lock().unwrap();       // 父链初始化
                    let copy_chain = parentchain.clone();

                    let mut newline:Vec<H256> = Vec::new(); //初始化新hash列表
                    // 匹配不在链中的hash

                    // 如果 hashes中的值不在chain中，发送消息getblocks，请求块插入
                    // 遍历给的hash值
                    for hash in hashes
                    {   
                        if copy_chain.chain.contains_key(&hash) {   // 如果链中包含hash值
                            continue;   // 跳过，处理下一个
                        } else {    // 链中不包含hash值
                            newline.push(hash);     // 入栈请求队列
                            //debug!("New blocks: {}", hash); 
                        }
                    }
                    if newline.len() > 0{   // 如果请求队列不为空
                        //let line_cln = newline.clone();
                        //println!("Asking of new blocks from sender -- :{:?}", newline);
                        peer.write(Message::GetBlocks(newline));    // 向sender请求新块

                        //self.server.broadcast(Message::NewBlockHashes(line_cln));   //广播请求队列（视为新块加入）
                    }
                   
                }
                Message::GetBlocks(hashes) => {
                    let mut parentchain = chain.lock().unwrap();       // 父链初始化
                    let copy_chain = parentchain.clone();
                    //debug!("GetBlocks");
                    // 收到此条消息后，检查自己链中是否有符合hash值的块，有则发送
                    // 检查hashes包含的块是否在chain中，如果在，调用blocks插入
                    let mut existline:Vec<Block> = Vec::new();
                    for hash in hashes
                    {    // 遍历chain中block
                        
                        if copy_chain.chain.contains_key(&hash) {   // 当前被请求块在链中
                            let block_asked = copy_chain.chain.get(&hash).unwrap().clone();  // 找到对应块
                            existline.push(block_asked);    // 入栈到发送队列
                        }
                   }
                   if existline.len() > 0 {     // 如果有被请求的块，发送自己的块信息
                        peer.write(Message::Blocks(existline));
                   } else {
                       println!("Do not have asked block");
                   }
                   
                }
                Message::Blocks(blocks) => {
                    let mut parentchain = chain.lock().unwrap();       // 父链初始化
                    //debug!("received blocks ");
                    let copy_chain = parentchain.clone();
                    let orphan = Arc::clone(&self.orphan_chain);
                    let mut orphan_line = orphan.lock().unwrap();
                    //debug!("Blocks");
                    // 收到消息后（收到是块信息），判断是否存在链中
                    let mut newblocks:Vec<H256> = Vec::new();
                    let mut missingparent:Vec<H256> = Vec::new();
                    for block in blocks.iter() {
                        if copy_chain.chain.contains_key(&block.header.hash()) {    //如果链中有新到的块
                            continue;   //不作处理
                        } else if copy_chain.chain.contains_key(&block.header.parent_pointer) { //链中有块的父块
                            // 验证difficulty
                            if block.header.hash() <= copy_chain.chain.get(&block.header.parent_pointer).unwrap().header.difficulty {
                                // 1. 插入新块
                                println!("\n");
                                println!("///////////////===============================================");
                                debug!("New BLOCK received!, tip: {:?}, longest height: {:?}. parent: {:?}, hash: {:?}. Time: {:?}", parentchain.tip, parentchain.longest_height, block.header.parent_pointer, block.header.hash(),  block.header.timestamp);
                                parentchain.insert(block);  //插入
                                println!("===============================================///////////////");
                                println!("\n");
                                newblocks.push(block.header.hash()); //入栈待广播队列
                                
                                // 2. 判断新块是不是某orphan的父块
                                let mut start_block = block.clone();
                                loop {
                                    if orphan_line.contains_key(&start_block.header.hash()) { //是某一orphan的父块
                                        let orphan_here = orphan_line.get(&start_block.header.hash()).unwrap();
                                        println!("\n");
                                        println!("///////////////===============================================");
                                        println!("New BLOCK received!, tip: {:?}, longest height: {:?}. parent: {:?}, hash: {:?}. Time: {:?}", parentchain.tip, parentchain.longest_height, orphan_here.header.parent_pointer, orphan_here.header.hash(),  orphan_here.header.timestamp);
                                        println!("===============================================///////////////");
                                        println!("\n");
                                        parentchain.insert(&orphan_here.clone());     //插入此orphan块
                                        start_block = orphan_here.clone();          // 更新尝试父块

                                        newblocks.push(orphan_here.header.hash()); //入栈待广播队列
                                    } else {    // 新尝试块不是orphan中某块父块，跳出循环
                                        break;
                                    }
                                }
                            }
                        } else {    // 链中没有新块且没有父块
                            let blockcln = block.clone();
                            orphan_line.insert(block.header.parent_pointer,blockcln);   //入orphan到缓存，key设为父块哈希值
                            missingparent.push(block.header.parent_pointer);    // 入栈parent hash值
                        }
                    }
                    let pool_clone = parentchain.tx_mempool.clone();
                    for j in pool_clone.keys() {
                       let tx = parentchain.tx_mempool.get(j).unwrap().clone();
                        if check_tx(&tx, &parentchain.ledger_state) {
                            continue;
                        } else {
                            parentchain.tx_mempool.remove(j);
                        }
                    }

                    if missingparent.len() > 0 {
                        println!("not find parent block, get in parent block -- {:?}", missingparent);
                        peer.write(Message::GetBlocks(missingparent));
                    }
                    if newblocks.len() > 0 {
                        //println!("broadcasting new block -- {:?}", newblocks);
                                //peer.write(Message::NewBlockHashes(new_hash));
                                self.server.broadcast(Message::NewBlockHashes(newblocks)); 
                    }
                    
                }
                Message::NewTransactionHashes(txhashes) => {
                    
                    let parentchain = chain.lock().unwrap();       // 父链初始化
                    let copy_chain = parentchain.clone();
                    //debug!("Heared new txs : {:?}", txhashes);
                    
                    let mut calling_tx = Vec::new();
                    // 检查是否在mempool中，不在，请求

                    for tx_hs in txhashes.iter() {
                        // 链中的mempool含有此hash
                        if copy_chain.tx_mempool.contains_key(&tx_hs){
                            continue;
                        }
                        else {
                            calling_tx.push(tx_hs.clone());
                        }
                    }

                    if calling_tx.len() > 0 {
                        peer.write(Message::GetTransactions(calling_tx));    // 向sender请求新块
                    }
                }
                Message::GetTransactions(txhashes) => {
                    // 检查是否有tx
                    let parentchain = chain.lock().unwrap();       // 父链初始化
                    let copy_chain = parentchain.clone();
                    
                    //debug!("Asked for txs: {:?}", txhashes);
                    let mut exist_tx = Vec::new();

                    for tx_hs in txhashes.iter() {
                        if copy_chain.tx_mempool.contains_key(tx_hs) {
                            let tx_now = copy_chain.tx_mempool.get(tx_hs).unwrap().clone();
                            exist_tx.push(tx_now);
                        }
                    }

                    if exist_tx.len() > 0 {
                        peer.write(Message::Transactions(exist_tx));
                    }
                }
                Message::Transactions(txs) => {
                    let mut parentchain = chain.lock().unwrap();       // 父链初始化
                    //debug!("Received txs");
                    let copy_chain = parentchain.clone();
                    // 判断是否在缓存中，如果不在，check，并加入缓存
                    let mut new_tx = Vec::new();
                    //let mut miss_tx = Vec::new();
                    for i in txs.iter() {
                        if copy_chain.tx_mempool.contains_key(&i.hash()) {
                            continue;
                        } else {
                            if check_tx(i, &copy_chain.ledger_state.clone()) {
                                parentchain.tx_mempool.insert(i.hash(), i.clone());   // 加入缓存
                                debug!("New Confirmed Tx founded! Hash: {:?}", i.hash());
                                new_tx.push(i.hash());
                                
                            }
                        }
                    }

                    let pool_clone = parentchain.tx_mempool.clone();
                    for j in pool_clone.keys() {
                        let tx = parentchain.tx_mempool.get(j).unwrap().clone();
                        if check_tx(&tx, &parentchain.ledger_state) {
                            continue;
                        } else {
                            parentchain.tx_mempool.remove(j);
                        }
                    }
                    println!("TX pool size: {:?}", parentchain.tx_mempool.keys().len());

                    if new_tx.len()>0 {
                        self.server.broadcast(Message::NewTransactionHashes(new_tx)); 
                    }
                }
                Message::NewStateHashes(statehashes) => {
                    
                    let parentchain = chain.lock().unwrap();       // 父链初始化
                    //debug!("Heared new states: {:?}", statehashes);
                    let copy_chain = parentchain.clone();
                    let mut new_state = Vec::new();
                    for i in statehashes.iter() {
                        if copy_chain.ledger_state.contains_key(i) {
                            continue;
                        } else {
                            new_state.push(i.clone());
                        }
                    }
                    

                    if new_state.len() > 0 {
                        peer.write(Message::GetStates(new_state));
                    } //else {
                        //debug!("ledger_state now: {:?}", parentchain.ledger_state);
                    //}
                }
                Message::GetStates(statehashes) => {
                    let parentchain = chain.lock().unwrap();       // 父链初始化
                    //debug!("Asked new states: {:?}", statehashes);
                    let copy_chain = parentchain.clone();

                    let mut exist_state = HashMap::new();
                    for i in statehashes.iter() {
                        if copy_chain.ledger_state.contains_key(i) {
                            exist_state.insert(i.clone(), copy_chain.ledger_state.get(i).unwrap().clone());
                        }
                    }
                    if exist_state.clone().len() > 0 {
                        peer.write(Message::States(exist_state));
                    }

                }
                Message::States(states) => {
                    let mut parentchain = chain.lock().unwrap();       // 父链初始化
                    debug!("received new states: {:?}", states.keys());
                    let copy_chain = parentchain.clone();

                    let mut new_state = Vec::new();

                    for j in parentchain.ledger_state.keys() {
                        if states.contains_key(j) {
                            continue;
                        } else {
                            new_state.push(j.clone());
                        }
                        
                    }

                    for i in states.keys() {
                        let state = states.get(i).unwrap();
                        if copy_chain.ledger_state.contains_key(i) {
                            continue;
                        } else {
                            new_state.push(i.clone());
                            parentchain.ledger_state.insert(i.clone(), state.clone());
                            //debug!("New state Tx Synchronized! Hash: {:?}", i);
                        }
                    }
                    let mut exist = Vec::new();
                    for k in parentchain.ledger_state.keys() {
                        exist.push(k.clone());
                    }
                    println!("ledger_state now: {:?}", exist);
                    if new_state.len()>0 {
                        self.server.broadcast(Message::NewStateHashes(new_state)); 
                    }

                }
                Message::DeleteStateHashes(state) => {
                    let mut parentchain = chain.lock().unwrap();       // 父链初始化
                    let copy_chain = parentchain.clone();
                    for i in state.iter() {
                        if copy_chain.ledger_state.contains_key(i) {
                            parentchain.ledger_state.remove(i);
                            self.server.broadcast(Message::DeleteStateHashes(state.clone())); 
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
