use crate::network::server::Handle as ServerHandle;

use crate::blockchain::*;
use crate::block::*;
use crate::transaction::*;
use std::sync::Arc;
use std::sync::Mutex;
use rand::Rng;
use crate::crypto::hash::{H256, Hashable, H160, convert_to_h160};
use crate::crypto::merkle::*;
use chrono::prelude::*;
use std::collections::HashMap;
use crate::crypto::key_pair;
use ring::signature::{self, Ed25519KeyPair, Signature, KeyPair};
use log::{debug, warn};


use crate::network::message::Message;

use log::info;

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time;

use std::thread;

enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}
 
pub struct Context {
    /// Channel for receiving control signal
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    server: ServerHandle,
    block_chain: Arc<Mutex<Blockchain>>,        // added for blockchain's tip
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(
    server: &ServerHandle, blockchain: &Arc<Mutex<Blockchain>>, 
) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        block_chain: Arc::clone(blockchain),
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, lambda: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda))
            .unwrap();
    }

}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.miner_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn handle_control_signal(&mut self, signal: ControlSignal) {
        match signal {
            ControlSignal::Exit => {
                info!("Miner shutting down");
                self.operating_state = OperatingState::ShutDown;
            }
            ControlSignal::Start(i) => {
                info!("Miner starting in continuous mode with lambda {}", i);
                self.operating_state = OperatingState::Run(i);
            }
        }
    }

    fn miner_loop(&mut self) {
        let start_time = Local::now().timestamp();
        // 本循环是否生成新tx
         
        let mut rng = rand::thread_rng();

        let miner_key = key_pair::random();
        let miner_pk = pk_to_u8(&miner_key.public_key());
        let miner_add = pk_to_h160(&miner_pk);
        println!("miner's add: {:?}", miner_add);

        let mut keypair = HashMap::new();
        // 每个miner中存在的keypair
        for i in 0..4 {
            let key = key_pair::random();
            let pk = pk_to_u8(key.public_key());
            let add = pk_to_h160(&pk);
            keypair.insert(add, key);
        }

        println!("key pair: {:?}", keypair.keys()); //

        

        // 初始化给四个可花state
        let mut init_state_hash = Vec::new();
        for i in 0..2 {
            let rand_txhash:[u8;32] = rng.gen();
            let rd_txhash:H256 = rand_txhash.into();
            init_state_hash.push(rd_txhash.clone());
            let chain = Arc::clone(&self.block_chain);
            let mut parentchain = chain.lock().unwrap();
            parentchain.ledger_state.insert(rd_txhash, generate_state(&keypair));
        }

        // 同步初始state
        info!("Synchronizing Ledger State.");
        self.server.broadcast(Message::NewStateHashes(init_state_hash.clone())); 

        println!("state: {:?}", init_state_hash);
        let mut try_time = 0;
        // main mining loop
        loop {
            // TODO: actual mining
            try_time += 1;

            if try_time %400 == 1{
                let chain = Arc::clone(&self.block_chain);
                let parentchain = chain.lock().unwrap();

                let mut state_before_mining = Vec::new();
                for i in parentchain.ledger_state.keys() {
                    state_before_mining.push(i.clone());
                }
                //info!("Synchronizing Ledger State.");
                self.server.broadcast(Message::NewStateHashes(state_before_mining)); 
            }
            
            // check duration
            let end_time = Local::now().timestamp();
            let gene_txs_flag = try_time % 3000;  
            let duration = 120; 
            if end_time - start_time >= duration {
                let chain = Arc::clone(&self.block_chain);
                let parentchain = chain.lock().unwrap();
                info!("Time's done. Total {:?} seconds.", duration);
                println!("Final Longest chain: {:?}",parentchain.longest_chain);
                let mut memp = Vec::new();
                let mut state = Vec::new();

                for i in parentchain.tx_mempool.keys() {
                    memp.push(i);
                }

                for j in parentchain.ledger_state.keys() {
                    state.push(j);
                }


                println!("Final tx mempool:  {:?}",memp);
                println!("Final ledger state{:?}",state);
                println!("----------------------- end ---------------------");
               return; 
            }

            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    self.handle_control_signal(signal);
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        self.handle_control_signal(signal);
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Miner control channel detached"),
                },
            }

            if let OperatingState::ShutDown = self.operating_state {
                return;
            }
            
            
            
            //------------------------- new transaction generating --------
            //generate txs
            
            if gene_txs_flag == 0 {
                let mut emp_vec = Vec::new();
                let mut rand_tx = generate_signed_txs();
                let ad_tx = rng.gen_range(0,5);
                let mut state = HashMap::new();
                if ad_tx != 4 {
                    let chain = Arc::clone(&self.block_chain);
                    let mut parentchain = chain.lock().unwrap();
                    let state_now = parentchain.ledger_state.clone();
                    let new_tx = generate_confirmed_signedtransaction(&state_now, &keypair);
                    parentchain.tx_mempool.insert(new_tx.clone().hash(), new_tx.clone());
                    state = parentchain.ledger_state.clone();
                    rand_tx = new_tx.clone();
                    emp_vec.push(new_tx.clone().hash());
                    info!("New Confirmed Tx Generated! Hash: {:?}", new_tx.clone().hash());
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
                } else {
                    let chain = Arc::clone(&self.block_chain);
                    let mut parentchain = chain.lock().unwrap();
                    let state_now = parentchain.ledger_state.clone();
                    let new_tx = generate_signed_txs();
                    //parentchain.tx_mempool.insert(new_tx.clone().hash(), new_tx.clone());
                    state = parentchain.ledger_state.clone();
                    rand_tx = new_tx.clone();
                    emp_vec.push(new_tx.clone().hash());
                    warn!("New Adversary Tx Generated! Hash: {:?}", new_tx.clone().hash());
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
                }
                if check_tx(&rand_tx, &state) {
                    self.server.broadcast(Message::NewTransactionHashes(emp_vec)); 
                }
                
                continue;
            } 
            // ----------------------------------------------------------------
            let chain = Arc::clone(&self.block_chain);
            let mut parentchain = chain.lock().unwrap();

            let parentblock = parentchain.chain.get(&parentchain.tip).unwrap(); // 找链上最长块
            let parentdiff = parentblock.header.difficulty; //difficulty
            let parentheight = parentblock.content.height;  //height
  
            // set timestamp
            let time_mill = Local::now().timestamp_millis();

            //------------------------------------------
            // bonus for miner
            let bonus_tx_for_miner = bonus_tx(&miner_key);

            // ---------------------------------------------------

            // set default transac
            let mut transac:Vec<H256> = Vec::new();
            let mut transac_dt: HashMap<H256, SignedTransaction> = HashMap::new();
            transac.push(bonus_tx_for_miner.hash());
            transac_dt.insert(bonus_tx_for_miner.hash(), bonus_tx_for_miner);

            // content
            let content = Content{
                transactions : transac.clone(),
                height : parentheight + 1,
                transaction_detail : transac_dt,
            };

            // head
            let head = Header{
                parent_pointer: parentchain.tip(),
                nonce: rng.gen(),
                difficulty: parentdiff,
                timestamp: time_mill,
                merkle_root: MerkleTree::new(&transac).root(),
            };
    
            let mut mined_block = Block{
                header: head,
                content: content,
            };

            let size:usize = 700;
            // 检查条件，插入块

            // 从mempool读一定数量txs，插入

            // loop, until max size
            
            let transactionmmp = parentchain.tx_mempool.clone();// mempool备份
            // 动态state
            let mut check_state = parentchain.ledger_state.clone();
            let mut check_mmp = transactionmmp.clone();

            for key in transactionmmp.keys() {
                // 循环插入txs
                let tx_waited = transactionmmp.get(key).unwrap();   //拿到SignedTransaction

                if !check_tx(tx_waited, &parentchain.ledger_state) {
                    parentchain.tx_mempool.remove(key);
                    check_mmp.remove(key);
                } 

                if check_tx(tx_waited, &check_state) { //如果当前tx合法，选中假定删除state
                    mined_block.content.transaction_detail.insert(tx_waited.clone().hash(),tx_waited.clone());
                    mined_block.content.transactions.push(tx_waited.clone().hash());
                    check_state.remove(&tx_waited.tx.input.prev_tx);
                    check_mmp.remove(key);
                }

                let new_merkleroot = MerkleTree::new(&(mined_block.content.transactions)).root();
                mined_block.header.merkle_root = new_merkleroot;

                //calc size now
                let size_now = bincode::serialize(&mined_block).unwrap();
                //println!("{:?}",size_now.len());
                let size_long = size_now.len();

                if size_long > size {
                    //println!("{:?}",mined_block.content.transactions.len());
                    break;
                }
            }

            let mut delete_state:Vec<H256> = Vec::new();

            if mined_block.hash() <= mined_block.header.difficulty {
                
                // 更新state
                
                for i in parentchain.ledger_state.keys() {
                    if check_state.contains_key(i) {
                        continue;
                    } else {
                        delete_state.push(i.clone());
                    }
                }

                parentchain.ledger_state = check_state; // 删除花了的
                for key in mined_block.content.transaction_detail.keys() {  //添加新生成的
                    let new_state = mined_block.content.transaction_detail.get(key).unwrap().clone();
                    if new_state.tx.input.prev_tx != [0;32].into() {
                        parentchain.ledger_state.insert(key.clone(),new_state.tx.output);
                    }   
                }
                // 更新mmp
                for mmptx in parentchain.tx_mempool.clone().keys() {
                    let tx_update = parentchain.tx_mempool.get(mmptx).unwrap();
                    if check_tx(tx_update, &parentchain.ledger_state) {
                        continue;
                    } else {
                        parentchain.tx_mempool.remove(mmptx);
                    }
                }
                // 符合条件，加txs，插入
                println!("\n");
                println!("///////////////===============================================");
                info!("new block mined!, tip: {:?}, longest height: {:?}. parent: {:?}, hash: {:?}. Time: {:?}", parentchain.tip, parentchain.longest_height, mined_block.header.parent_pointer, mined_block.header.hash(), mined_block.header.timestamp);
                
                parentchain.insert(&mined_block);
                println!("===============================================///////////////");
                println!("\n");

                //println!("block's tx: {:?}", mined_block.content.transaction_detail);
                //println!(" Tx mempool(txs' hash): {:?} \n Ledger state: {:?}",  parentchain.tx_mempool.keys(),parentchain.ledger_state);
                let mut braod_line:Vec<H256> = Vec::new();
                braod_line.push(mined_block.hash()); 
                //println!("broadcasting new blocks -- {:?}", braod_line);
                self.server.broadcast(Message::NewBlockHashes(braod_line)); 
                self.server.broadcast(Message::DeleteStateHashes(delete_state));
            }

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
        
    }
}
