use serde::{Serialize,Deserialize};
use crate::crypto::hash::{H256, Hashable, H160, convert_to_h160};
use ring::signature::{self, Ed25519KeyPair, Signature, KeyPair};
use rand::Rng;
use ring::rand::{SecureRandom, SystemRandom};
use crate::crypto::key_pair;
use std::collections::HashMap;



#[derive(Hash, Eq, PartialEq,Debug, Default,Clone, Serialize, Deserialize)]
pub struct TxInput {
    pub prev_tx: H256,      //用来找上一个tx的input和output
    pub index : u32,
}

#[derive(Hash, Eq, PartialEq,Debug, Default,Clone, Serialize, Deserialize)]
pub struct TxOutput {
    pub value : u32,
    pub address: H160,
    //pub receiver_pk: [u8;32],
}

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    pub input : TxInput,
    pub output : TxOutput,
}

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct SignedTransaction {
    pub tx : Transaction,
    pub pk : [u8;32],
    pub signature1 : [u8;32],
    pub signature2 : [u8;32],
}

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize, Debug, Default, Clone)]
pub struct BinSignedTransaction {
    pub tx : Transaction,
    pub pk : H256,
    pub signature : H256,
}

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct State {
    pub value : u32,
    pub address : H160,
    pub pk : [u8;32],
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    key.sign(bincode::serialize(&t).unwrap().as_ref())
    // unimplemented!() 
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &<Ed25519KeyPair as KeyPair>::PublicKey, signature: &Signature) -> bool {
    let peer_public_key_bytes = public_key.as_ref();//u8格式
    let peer_public_key =
    signature::UnparsedPublicKey::new(&signature::ED25519, peer_public_key_bytes);
    peer_public_key.verify(bincode::serialize(&t).unwrap().as_ref(), signature.as_ref()).is_ok()
    // unimplemented!()
}

pub fn verify_u8(t: &Transaction, public_key: &[u8;32], signature1: &[u8;32], signature2: &[u8;32]) -> bool {
    let peer_public_key =
    signature::UnparsedPublicKey::new(&signature::ED25519, &public_key[0..32]);
    let mut sig = [0;64];
    for i in 0..32 {
        sig[i] = signature1[i];
        sig[i+32] = signature2[i];
    }
    
    peer_public_key.verify(bincode::serialize(&t).unwrap().as_ref(), &sig[0..64]).is_ok()
    // unimplemented!()
}

 pub fn bonus_tx (key: &Ed25519KeyPair) -> SignedTransaction {
     let public_key = key.public_key();
     let rec_pk = pk_to_u8(public_key);
     let add = pk_to_h160(&rec_pk);    //address

     let input = TxInput{prev_tx:[0;32].into(), index: 0,};
     let output = TxOutput{value: 200000000, address: add, }; //receiver_pk : rec_pk,// mining fee = 2BTC, for miner

     let tx = Transaction{input: input, output: output,};

     let signature = sign(&tx, key);
     let (signature1, signature2) = sig_to_2_u8(&signature);

     SignedTransaction{
         tx: tx,
         pk: pk_to_u8(&public_key),
         signature1:signature1,
         signature2:signature2,
     }
 }

pub fn pk_to_h160(public_key: &[u8;32]) -> H160 {
    let pk_h256:H256 = ring::digest::digest(&ring::digest::SHA256, public_key).into();
    convert_to_h160(pk_h256)
}

pub fn pk_to_u8(public_key: &<Ed25519KeyPair as KeyPair>::PublicKey) -> [u8;32] {
    let pk = public_key.as_ref();
    let mut pk_u8 = [0;32];
    for i in 0..32 {
        pk_u8[i] = pk[i];
    }
    pk_u8
}

pub fn sig_to_2_u8(signature: &Signature) -> ([u8;32],[u8;32]) {
    let mut sig1  = [0;32];
    let mut sig2  = [0;32];
    for i in 0..32 {
        sig1[i] = signature.as_ref()[i];
    }
    for j in 32..64 {
        sig2[j-32] = signature.as_ref()[j];
    }
    (sig1, sig2)
}



pub fn check_tx(t: &SignedTransaction, state: &HashMap<H256,TxOutput>) -> bool {
    // 验证tx的合理性
    // 1. 检查sig是否是pk签的
    let signature1 = t.signature1;
    let signature2 = t.signature2;
    let pk = t.clone().pk;
    let tx = t.clone().tx;
    // check sig
    let sig_check = verify_u8(&tx, &pk, &signature1, &signature2);
    // check input 是否可花
    let input_check:bool = state.contains_key(&(tx.input.prev_tx)) || tx.input.prev_tx == [0;32].into();
    let mut owner_check = false;
    let mut value_check = false;
    // check 所有权与金额
    if input_check == true {
        let input_value = state.get(&(tx.input.prev_tx)).unwrap(); 
        let add_h160 = pk_to_h160(&pk);
        owner_check = add_h160==input_value.address;
        value_check = input_value.value >= tx.output.value;
    }
    
    input_check && owner_check && sig_check && value_check
}

pub fn check_tx_missprev(t: &SignedTransaction, state: &HashMap<H256,TxOutput>) -> bool {
    // 验证tx的合理性
    // 1. 检查sig是否是pk签的
    // 验证tx的合理性
    // 1. 检查sig是否是pk签的
    let signature1 = t.signature1;
    let signature2 = t.signature2;
    let pk = t.clone().pk;
    let tx = t.clone().tx;
    // check sig
    let sig_check = verify_u8(&tx, &pk, &signature1, &signature2);
    // check input 是否可花
    let input_check:bool = state.contains_key(&(tx.input.prev_tx)) || tx.input.prev_tx == [0;32].into();
    let mut owner_check = false;
    let mut value_check = false;
    // check 所有权与金额
    if input_check == true {
        let input_value = state.get(&(tx.input.prev_tx)).unwrap(); 
        let add_h160 = pk_to_h160(&pk);
        owner_check = add_h160==input_value.address;
        value_check = input_value.value >= tx.output.value;
    }
    
    !input_check && owner_check && sig_check && value_check
}

pub fn generate_random_transaction() -> Transaction {
    // generate transaction
    let mut rng = rand::thread_rng();
    let bf_tx: [u8;32] = rng.gen();     // nonce generated
    let bf_idx: u32 = 0;     // nonce generated
    let Value : u32 = rng.gen();
    let rec_pk : [u8;32] = rng.gen();
    let add = pk_to_h160(&rec_pk);
    let mut input = TxInput{prev_tx :bf_tx.into(), index : bf_idx};
    let mut output = TxOutput{value:Value, address: add.into(), };//receiver_pk : rec_pk

    Transaction{
        input : input,
        output : output,
    }
}

pub fn generate_confirmed_signedtransaction(
    state: &HashMap<H256, TxOutput>, 
    keypair: &HashMap<H160, Ed25519KeyPair>) -> SignedTransaction {
    // generate transaction
    let mut rng = rand::thread_rng();
    let rec_key_num = rng.gen_range(0,keypair.len());
    let mut hash:H256 = [0;32].into(); 
    let mut num = 0;
    let mut value:u32 = 0;
    let mut add:H160 = [0;20].into();

    //选state中一个作为输入
    let mut self_state = state.clone();
    for j in state.keys() {
        let address_of_state = state.get(j).unwrap().address;
        if keypair.contains_key(&address_of_state) {
            continue;
        } else {
            self_state.remove(j);
        }
    }
    let rand_hash = rng.gen_range(1,self_state.len()); // miner's state
    for i in self_state.keys() {
        if num == rand_hash {
            hash = i.clone();
            value = state.get(i).unwrap().value;
            //pk = state.get(i).unwrap().receiver_pk;
            add = state.get(i).unwrap().address;
        }
        num += 1;
    }
    let key = keypair.get(&add).unwrap();
    let pk = pk_to_u8(key.public_key());

    let mut rec_pk = [0;32];
    let mut rec_add:H160 = [0;20].into();
    let mut index = 0;
    for j in keypair.keys() {
        if index == rec_key_num {
            rec_pk = pk_to_u8(keypair.get(j).unwrap().public_key());
            rec_add = pk_to_h160(&rec_pk);
        }
        index += 1;
    }

    
    let input = TxInput{prev_tx :hash, index : 0};
    let output = TxOutput{value:value, address: rec_add,}; // receiver_pk: rec_pk

    let tx = Transaction{
        input : input,
        output : output,
    };

    let signature = sign(&tx,key);
    let (sig1,sig2) = sig_to_2_u8(&signature);

    SignedTransaction{
        tx:tx,
        pk:pk,
        signature1: sig1,
        signature2: sig2,
    }
}

pub fn generate_advers_signedtransaction(
    state: &HashMap<H256, TxOutput>, 
    keypair: &HashMap<H160, Ed25519KeyPair>) -> SignedTransaction {
    // generate transaction
    let mut rng = rand::thread_rng();
    let rand_hash = rng.gen_range(0,state.len()) - 1;
    let rec_key_num = rng.gen_range(0,keypair.len() - 1);
    let mut hash:H256 = [0;32].into(); 
    let mut num = 0;
    let mut value:u32 = 0;

    let mut add:H160 = [0;20].into();
    
    //选state中一个作为输入
    for i in state.keys() {
        if num == rand_hash {
            hash = i.clone();
            value = state.get(i).unwrap().value;
            //pk = state.get(i).unwrap().receiver_pk;
            add = state.get(i).unwrap().address;
        }
        num += 1;
    }
    let key = key_pair::random();
    let pk = pk_to_u8(key.public_key());

    let mut rec_pk = [0;32];
    let mut rec_add:H160 = [0;20].into();
    let mut index = 0;
    for j in keypair.keys() {
        if index == rec_key_num {
            rec_pk = pk_to_u8(key.public_key());
            rec_add = pk_to_h160(&rec_pk);
        }
        index += 1;
    }

    
    let input = TxInput{prev_tx :hash, index : 0};
    let output = TxOutput{value:value, address: rec_add,}; // receiver_pk: rec_pk

    let tx = Transaction{
        input : input,
        output : output,
    };

    let signature = sign(&tx,&key);
    let (sig1,sig2) = sig_to_2_u8(&signature);

    SignedTransaction{
        tx:tx,
        pk:pk,
        signature1: sig1,
        signature2: sig2,
    }
}


pub fn generate_state(keypair: &HashMap<H160, Ed25519KeyPair>) -> TxOutput {
    let mut rng = rand::thread_rng();
    let rand_key = rng.gen_range(0,keypair.len());
    let mut pk = [0;32];
    let mut idx = 0;
    for i in keypair.keys() {
        if idx == rand_key {
            pk = pk_to_u8(keypair.get(i).unwrap().public_key());
            break;
        }
        idx += 1;
    }
    let value:u32 = rng.gen();
    let add = pk_to_h160(&pk);
    TxOutput{value:value, address: add, } //receiver_pk:pk,
}

pub fn generate_signed_txs() -> SignedTransaction {
    let t = generate_random_transaction();
    let key = key_pair::random();
    let signature = sign(&t, &key);
    let (sig1,sig2) = sig_to_2_u8(&signature);
    SignedTransaction {
        tx: t,
        signature1: sig1,
        signature2: sig2,
        pk: pk_to_u8(key.public_key()),
    }

}


#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::crypto::key_pair;

    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, &(key.public_key()), &signature));
    }
}

#[cfg(any(test, test_utilities))]
mod tests_Liz {
    use super::*;
    use crate::crypto::key_pair;

    #[test]
    fn sign_verify() {
        let tx = generate_signed_txs();
        let t = tx.clone().tx;
        let key = tx.clone().pk;
        let mut pk_u8 = [0;32];
        let mut sig1  = [0;32];
        let mut sig2  = [0;32];
        for i in 0..32 {
            pk_u8[i] = key.as_ref()[i];
            sig1[i] = tx.signature1[i];
            sig2[i] = tx.signature2[i];
            
        }

        let txu8 = SignedTransaction{
            tx:t.clone() ,
            pk: pk_u8,
            signature1: sig1,
            signature2: sig2,
        };
        //println!{"{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}", key.as_ref(), pk_u8,  tx.signature1,sig1, tx.signature2 ,sig2};

        let mut out = txu8.tx.output;
        out.value = out.value;

        //println!{"-----------------------------{:?}",verify_u8(&t, &pk_u8,&sig1,&sig2 )};

        assert!(verify_u8(&t, &pk_u8,&sig1,&sig2 ), true);

    }

    #[test]
    fn check_tx_verify() {
        let mut rng = rand::thread_rng();

        let mut keypair = HashMap::new();
        // 每个miner中存在的keypair
        for i in 0..5 {
            let key = key_pair::random();
            let pk = pk_to_u8(key.public_key());
            let add = pk_to_h160(&pk);
            keypair.insert(add, key);
        }

        let mut state:HashMap<H256,TxOutput> = HashMap::new();
        for i in 0..4 {
            let hash:[u8;32] = rng.gen();
            let hash256: H256 = hash.into();
            state.insert(hash256, generate_state(&keypair));
        }
        let mut tx  = generate_confirmed_signedtransaction(&state, &keypair);
        //tx.pk = [1;32];
        let check = check_tx(&tx, &state);

        loop {
            let rand = rng.gen_range(0,state.len());
            println!("{:?}", rand);
        }

      


        assert!(check, true);

    }
}


