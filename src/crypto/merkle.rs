use std::{convert::{self, From, TryInto}}; //intrinsics::{powf32, powif32}
use ring::{digest, test};

use super::hash::{Hashable, H256};

#[derive(Debug, Default, Clone)]
struct MerkleTreeNode {
    left: Option<Box<MerkleTreeNode>>,
    right: Option<Box<MerkleTreeNode>>,
    hash: H256,
}

/// A Merkle tree.
#[derive(Debug, Default)]
pub struct MerkleTree {

    root: MerkleTreeNode,
    level_count: usize, // how many levels the tree has
}

/// Given the hash of the left and right nodes, compute the hash of the parent node.
fn hash_children(left: &H256, right: &H256) -> H256 {
    let left_node = left.as_ref();
    let right_node = right.as_ref();
    let new_node = [left_node, right_node].concat();
    let hashed_newnode = ring::digest::digest(&digest::SHA256, &new_node);
    let test_num = hashed_newnode.as_ref();
    let mut sum: [u8; 32] = [0; 32];
    let mut i = 0;
    while i < test_num.len() {

        sum[i] = test_num[i];

        i = i + 1;
    }
    From::from(sum)
    // unimplemented!();
}

/// Duplicate the last node in `nodes` to make its length even.
fn duplicate_last_node(nodes: &mut Vec<Option<MerkleTreeNode>>) {
    nodes.push(nodes.last().cloned().unwrap());
    // unimplemented!();
}

impl MerkleTree {
    pub fn new<T>(data: &[T]) -> Self where T: Hashable, {
        // 输入一组data
        assert!(!data.is_empty());
        // create the leaf nodes:
        // 生成叶子节点，是一个vector容器，装着结构体MerKleTreeNode，现在是空的
        let mut curr_level: Vec<Option<MerkleTreeNode>> = Vec::new();
        // push data数据到vec中
        for item in data {
            curr_level.push(Some(MerkleTreeNode { hash: item.hash(), left: None, right: None }));
        }
        let mut level_count = 1;        // level计数器
        
        // create the upper levels of the tree:
        while curr_level.len() > 1 {
            // Whenever a level of the tree has odd number of nodes, duplicate the last node to make the number even:
            if curr_level.len() % 2 == 1 {
                duplicate_last_node(&mut curr_level); // TODO: implement this helper function
            }

          
            assert_eq!(curr_level.len() % 2, 0); // make sure we now have even number of nodes.

            let mut next_level: Vec<Option<MerkleTreeNode>> = Vec::new();   // 每一层新建一个上层Hash值的vec容器
            for i in 0..curr_level.len() / 2 {      // 当前节点数一半
                let left = curr_level[i * 2].take().unwrap();       // 分别取当前层对应节点位置处的值
                let right = curr_level[i * 2 + 1].take().unwrap();
                let hash = hash_children(&left.hash, &right.hash); // TODO: implement this helper function
                
                next_level.push(Some(MerkleTreeNode { hash: hash, left: Some(Box::new(left)), right: Some(Box::new(right)) })); // 生成新节点结构体，存到上层
        
            }
            curr_level = next_level; 
            level_count += 1;

        } 
        MerkleTree {
            root: curr_level[0].take().unwrap(),    // 取当前最顶层的Hash值为数的root
            level_count: level_count,               // 取层数
        }
    }

    // given a merkle tree, return the root. The computation of the root is inside new(), this function should just return the root.
    pub fn root(&self) -> H256 {
        self.root.hash
        // unimplemented!()
    }

    /// Returns the Merkle Proof of data at index i
    pub fn proof(&self, index: usize) -> Vec<H256> {
        let base: i32 = 2;
        // 2^level方，计算节点总数
        let leaf_number: usize = base.pow((self.level_count - 1).try_into().unwrap()).try_into().unwrap();
        let mut mid_number = (leaf_number + 1) / 2;    // 初始中间位置值
        let mut level_now = self.level_count;    // 取 root所在层数为初始值

        let mut node = self.root.clone();
        let mut neighbors: Vec<H256> = Vec::new();      // 向下索引的路径

        // 不是叶子节点时
        while level_now > 1 {
            // left node
            if mid_number > index {
                let node_another = *node.right.take().unwrap();
                node = *node.left.take().unwrap();                 // 更新root
                neighbors.push(node_another.hash);                  // 入栈相邻节点hash
            }
            else {
                let node_another = *node.left.take().unwrap();
                node = *node.right.take().unwrap();
                neighbors.push(node_another.hash);
            }
            mid_number = (mid_number + 1) / 2;
            level_now -= 1;
            
        }
        neighbors
        // unimplemented!()
    }

}

/// Verify that the datum hash with a vector of proofs will produce the Merkle root. Also need the
/// index of datum and `leaf_size`, the total number of leaves.

/// given a root, a hash of datum, a proof (a vector of hashes), an index of that datum (same index in proof() function), 
///  and a leaf_size (the length of leaves/data in new() function), returns whether the proof is correct.

pub fn verify(root: &H256, datum: &H256, proof: &[H256], index: usize, leaf_size: usize) -> bool {
    let levelnum = (leaf_size + 1) /2 + 1;
    let mut concatresult: Vec<u8> = Vec::new();
    let mut levelcurr = 1;
    let datum_u8 = datum.as_ref();

    while levelcurr < levelnum {    // 没有到根节点时，计算两两叶节点的根节点
        let proof_now = proof[proof.len() - levelcurr].as_ref();
        if index % 2 == 0 {    //左端点
            concatresult = [datum_u8, proof_now].concat();
        }
        else {
            concatresult = [proof_now, datum_u8].concat();
        }
        levelcurr += 1;
    }
    let concath256 = ring::digest::digest(&digest::SHA256, &concatresult);

    let root_proofted = concath256.as_ref();
    let mut sum: [u8; 32] = [0; 32];
    let mut i = 0;

    while i < root_proofted.len() {

        sum[i] = root_proofted[i];

        i = i + 1;
    }
    let root_h256: H256 = From::from(sum);
    if &root_h256 == root {
        return true;
    }else {
        return false;
    }
    // unimplemented!()
}

#[cfg(test)]
mod tests {
    
    use crate::crypto::hash::H256;
    use super::*;

    macro_rules! gen_merkle_tree_data {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    #[test]
    fn root() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        // 
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        assert_eq!(
            root,
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a 0ffa ea24 8de6 cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9 baa4 dcff 268a 4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a1216 2f28 7412 df3e c920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters


    }

    #[test]
    fn proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert_eq!(proof,
                   vec![hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into()]
        );
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
    }

    #[test]
    fn verifying() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));
    }
}
