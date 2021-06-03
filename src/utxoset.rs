use crate::block::Block;
use crate::blockchain::ForkTuple;
use crate::slip::{OutputSlip, SlipID};
use crate::transaction::Transaction;
use secp256k1::PublicKey;
use std::collections::HashMap;
use std::str::FromStr;

/// 
#[derive(Debug, Clone)]
enum LongestChainSpentStatus {
    Unspent,
    Spent(u64),
}

/// 
#[derive(Debug, Clone)]
enum ForkSpentStatus {
    ForkUnspent([u8; 32]),
    PotentialForkSpent([u8; 32]),
}

/// 
#[derive(Debug, Clone)]
struct SlipSpentStatus {
    output_slip: OutputSlip,
    longest_chain_status: Option<LongestChainSpentStatus>,
    fork_status: Vec<ForkSpentStatus>,
}

impl SlipSpentStatus {
    pub fn new_on_longest_chain(output_slip: OutputSlip, longest_chain_status: LongestChainSpentStatus) -> Self {
        SlipSpentStatus {
            output_slip: output_slip,
            longest_chain_status: Some(longest_chain_status),
            fork_status: vec![],
        }
    }
    
    pub fn new_on_fork(output_slip: OutputSlip, fork_status: ForkSpentStatus) -> Self {
        SlipSpentStatus {
            output_slip: output_slip,
            longest_chain_status: None,
            fork_status: vec![fork_status],
        }
    }
}


/// A hashmap storing Slips TODO fix this documentation once we've settled on what
/// data structures actually belong here.
#[derive(Debug, Clone)]
pub struct UtxoSet {
    shashmap: HashMap<SlipID, SlipSpentStatus>,
}

impl UtxoSet {
    /// Create new `UtxoSet`
    pub fn new() -> Self {
        UtxoSet {
            shashmap: HashMap::new(),
        }
    }

    pub fn roll_back_on_potential_fork(&mut self, block: &Block) {
        block
            .transactions()
            .iter()
            .for_each(|tx| self.fork_unspend_transaction(tx, block));
    }

    pub fn roll_forward_on_potential_fork(&mut self, block: &Block) {
        // spendoutputs and spend the inputs
        block
            .transactions()
            .iter()
            .for_each(|tx| self.fork_spend_transaction(tx, block));
    }

    pub fn roll_back(&mut self, block: &Block) {
        // unspend outputs and spend the inputs
        block
            .transactions()
            .iter()
            .for_each(|tx| self.unspend_transaction(tx, block));
    }

    pub fn roll_forward(&mut self, block: &Block) {
        block
            .transactions()
            .iter()
            .for_each(|tx| self.spend_transaction(tx, block));
    }
    // 
    // pub fn is_slip_spent(&self, slip_id: &SlipID) -> bool {
    //     match self.shashmap.get(slip_id) {
    //         Some(value) => match value {
    //             UtxoSetValue::Spent(_) => true,
    //             UtxoSetValue::PotentialForkSpent(_) => true,
    //             _ => false,
    //         },
    //         None => true,
    //     }
    // }
    /// Return the `Block` id based on `OutputSlip`
    ///
    /// * `slip` - `&OutputSlip` as key
    // pub fn slip_block_id(&self, slip_id: &SlipID) -> &OutputSlip {
    //     self.utxo_hashmap.get(slip_id).unwrap()
    // }

    /// Returns true if the slip has been seen in the blockchain
    pub fn is_slip_spent_at_block(&self, slip_id: &SlipID, _block: &Block, fork_tuple: &ForkTuple) -> bool {
        // match self.shashmap.get(slip_id) {
        //     Some(value) => match value {
        //         UtxoSetValue::Spent(_) => true,
        //         UtxoSetValue::PotentialForkSpent(_) => true,
        //         _ => false,
        //     },
        //     None => true,
        // };
        // if Slip is Unspent, return true
        // else if Slip is ForkUnspent, find common ancestor and check if the hash in
        // shashmap ForkUnspent is in any of the ancestors before common_ancestor
        //
        // let common_ancestor = BLOCKCHAIN.find_common_ancestor_in_longest_chain();
        // let next_block = block;
        // while next_block.previous_block_hash() != common_ancestor.hash() {
        //
        //     if(next_block.hash() == common_ancestor.hash()) {
        //         // cheese it
        //     }
        //     next_block = block.previous_block_hash();
        // }
        false
    }

    /// Returns true if the SlipOutput found in the utxoset matches the SlipOutput
    pub fn get_total_for_slips(&self, _slip_ids: Vec<SlipID>) -> u64 {
        100
    }

    /// Returns true if the SlipOutput found in the utxoset matches the SlipOutput
    pub fn get_sender_for_slips(&self, _slip_ids: Vec<SlipID>) -> Option<PublicKey> {
        Some(
            PublicKey::from_str(
                "0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072",
            )
            .unwrap(),
        )
    }

    pub fn output_slip_from_slip_id(&self, slip_id: &SlipID) -> Option<&OutputSlip> {
        match self.shashmap.get(slip_id) {
             Some(slip_output) => Some(&slip_output.output_slip),
             None => None,
        } 
    }

    /// Insert the inputs of a `Transaction` with the `Block` id
    ///
    /// * `tx` - `Transaction` which the inputs are inserted into `HashMap`
    fn spend_transaction(&mut self, tx: &Transaction, block: &Block) {
        // tx.core
        //     .inputs()
        //     .iter()
        //     .enumerate()
        //     .for_each(|(idx, _input)| {
        //         let slip_id = SlipID::new(*tx.signature(), idx as u64);
        //         // self.shashmap.remove(&slip_id);
        //         self.shashmap
        //             .insert(slip_id.clone(), UtxoSetValue::Spent(block.id()));
        //         // self.utxo_hashmap.remove(&slip_id);
        //     });
        // 
        // tx.core
        //     .outputs()
        //     .iter()
        //     .enumerate()
        //     .for_each(|(idx, output)| {
        //         let slip_id = SlipID::new(*tx.signature(), idx as u64);
        //         self.shashmap.insert(slip_id.clone(), UtxoSetValue::Unspent);
        //         self.utxo_hashmap.insert(slip_id, *output);
        //     });
    }
    
    fn unspend_transaction(&mut self, tx: &Transaction, _block: &Block) {
        tx.core.outputs().iter().enumerate().for_each(|(index, output)| {
            let slip_id = SlipID::new(tx.core.hash(), index as u64);
            self.shashmap.entry(slip_id)
               .and_modify(|slip_spent_status| {
                   slip_spent_status.longest_chain_status = Some(LongestChainSpentStatus::Unspent);
                })
               .or_insert(SlipSpentStatus::new_on_longest_chain(output.clone(), LongestChainSpentStatus::Unspent));
        });
    }
    fn unspend_transaction_on_fork(&mut self, tx: &Transaction, _block: &Block) {
        tx.core.outputs().iter().enumerate().for_each(|(index, output)| {
            let slip_id = SlipID::new(tx.core.hash(), index as u64);
            self.shashmap.entry(slip_id)
               .and_modify(|slip_spent_status| {
                   slip_spent_status.longest_chain_status = Some(LongestChainSpentStatus::Unspent);
                })
               .or_insert(SlipSpentStatus::new_on_fork(output.clone(), ForkSpentStatus::ForkUnspent(block.hash().clone()));
        });
    }

    fn fork_spend_transaction(&mut self, tx: &Transaction, block: &Block) {
        // tx.core
        //     .inputs()
        //     .iter()
        //     .enumerate()
        //     .for_each(|(idx, _input)| {
        //         let slip_id = SlipID::new(*tx.signature(), idx as u64);
        //         // self.shashmap.remove(&slip_id);
        //         self.shashmap.insert(
        //             slip_id.clone(),
        //             UtxoSetValue::PotentialForkSpent(block.hash()),
        //         );
        //         // self.utxo_hashmap.remove(&slip_id);
        //     });
        // 
        // tx.core
        //     .outputs()
        //     .iter()
        //     .enumerate()
        //     .for_each(|(idx, output)| {
        //         let slip_id = SlipID::new(*tx.signature(), idx as u64);
        //         self.shashmap.insert(
        //             slip_id.clone(),
        //             UtxoSetValue::ForkUnspent(block.hash()),
        //         );
        //         self.utxo_hashmap.insert(slip_id, *output);
        //     });
    }

    /// Remove the inputs of a `Transaction` with the `Block` id
    ///
    /// * `tx` - `Transaction` where inputs are inserted, and outputs are removed
    fn fork_unspend_transaction(&mut self, tx: &Transaction, block: &Block) {
        // tx.core.inputs().iter().for_each(|input| {
        //     self.shashmap
        //         .insert(*input, UtxoSetValue::ForkUnspent(block.hash()));
        // });
        // 
        // tx.core
        //     .outputs()
        //     .iter()
        //     .enumerate()
        //     .for_each(|(idx, _output)| {
        //         let slip_id = &SlipID::new(*tx.signature(), idx as u64);
        //         self.shashmap.remove(&slip_id);
        //     });
    }
}

#[cfg(test)]
mod test {
    //
    // use super::*;
    // use crate::{
    //     keypair::Keypair,
    //     slip::{Slip, SlipType},
    //     transaction::{Transaction, TransactionType},
    // };
    // use std::collections::HashMap;
    //
    // #[test]
    // fn shashmap_test() {
    //     let shashmap = UtxoSet::new();
    //     assert_eq!(shashmap.utxo_hashmap, HashMap::new());
    // }
    //
    // #[test]
    // fn shashmap_insert_test() {
    //     let mut shashmap = UtxoSet::new();
    //     let keypair = Keypair::new();
    //     let slip = Slip::new(keypair.public_key().clone(), SlipType::Normal, 0);
    //     shashmap.insert(slip, 0);
    //     assert!(shashmap.utxo_hashmap.contains_key(&slip));
    // }
    //
    // #[test]
    // fn shashmap_insert_new_transaction_test() {
    //     let mut shashmap = UtxoSet::new();
    //
    //     let public_key: PublicKey = PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072").unwrap();
    //     let mut tx_body = Transaction::new(TransactionType::Normal, public_key);
    //     let mut tx Transaction::add_signature(tx_body, &Signature::from_compact(&[0; 64]).unwrap());
    //
    //     let keypair = Keypair::new();
    //     let output_slip = Slip::new(keypair.public_key().clone(), SlipType::Normal, 0);
    //
    //     tx.add_output(output_slip);
    //
    //     shashmap.insert_new_transaction(&tx);
    //
    //     assert!(shashmap.utxo_hashmap.contains_key(&output_slip));
    //     assert_eq!(shashmap.utxo_hashmap.get(&output_slip).unwrap(), &-1);
    // }
    //
    // #[test]
    // fn shashmap_spend_transaction_test() {
    //     let mut shashmap = UtxoSet::new();
    //     let public_key: PublicKey = PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072").unwrap();
    //     let mut tx_body = Transaction::new(TransactionType::Normal, public_key);
    //     let mut tx Transaction::add_signature(tx_body, &Signature::from_compact(&[0; 64]).unwrap());
    //     let keypair = Keypair::new();
    //     let input_slip = Slip::new(keypair.public_key().clone(), SlipType::Normal, 0);
    //
    //     tx.add_input(input_slip);
    //
    //     shashmap.spend_transaction(&tx, 0);
    //
    //     assert!(shashmap.utxo_hashmap.contains_key(&input_slip));
    //     assert_eq!(shashmap.utxo_hashmap.get(&input_slip).unwrap(), &0);
    // }
    //
    // #[test]
    // fn shashmap_unspend_transaction_test() {
    //     let mut shashmap = UtxoSet::new();
    //     let public_key: PublicKey = PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072").unwrap();
    //     let mut tx_body = Transaction::new(TransactionType::Normal, public_key);
    //     let mut tx Transaction::add_signature(tx_body, &Signature::from_compact(&[0; 64]).unwrap());
    //
    //     let keypair = Keypair::new();
    //     let input_slip = Slip::new(keypair.public_key().clone(), SlipType::Normal, 0);
    //
    //     tx.add_input(input_slip);
    //
    //     shashmap.unspend_transaction(&tx);
    //
    //     assert!(shashmap.utxo_hashmap.contains_key(&input_slip));
    //     assert_eq!(shashmap.utxo_hashmap.get(&input_slip).unwrap(), &-1);
    // }
    //
    // #[test]
    // fn shashmap_spend_slip_test() {
    //     let mut shashmap = UtxoSet::new();
    //
    //     let keypair = Keypair::new();
    //     let input_slip = Slip::new(keypair.public_key().clone(), SlipType::Normal, 0);
    //
    //     shashmap.spend_slip(&input_slip, 0);
    //
    //     assert!(shashmap.utxo_hashmap.contains_key(&input_slip));
    //     assert_eq!(shashmap.utxo_hashmap.get(&input_slip).unwrap(), &0);
    // }
    //
    // #[test]
    // fn shashmap_unspend_slip_test() {
    //     let mut shashmap = UtxoSet::new();
    //
    //     let keypair = Keypair::new();
    //     let input_slip = Slip::new(keypair.public_key().clone(), SlipType::Normal, 0);
    //
    //     shashmap.unspend_slip(&input_slip);
    //
    //     assert!(shashmap.utxo_hashmap.contains_key(&input_slip));
    //     assert_eq!(shashmap.utxo_hashmap.get(&input_slip).unwrap(), &-1);
    // }
    //
    // #[test]
    // fn shashmap_slip_block_id_test() {
    //     let mut shashmap = UtxoSet::new();
    //
    //     let keypair = Keypair::new();
    //     let slip = Slip::new(keypair.public_key().clone(), SlipType::Normal, 0);
    //     shashmap.insert(slip, 1);
    //
    //     match shashmap.slip_block_id(&slip) {
    //         Some(id) => assert_eq!(id, &1),
    //         _ => assert!(false),
    //     }
    // }
}
