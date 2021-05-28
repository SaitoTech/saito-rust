use crate::slip::Slip;
use crate::transaction::Transaction;
use std::collections::HashMap;

/// A hashmap storing the byte arrays of `Slip`s as keys
/// with the `Block` ids as values. This is used to enforce when
/// `Slip`s have been spent in the network
#[derive(Debug, Clone)]
pub struct Shashmap {
    utxo_hashmap: HashMap<Slip, i64>,
}

impl Shashmap {
    /// Create new `Shashmap`
    pub fn new() -> Self {
        Shashmap {
            utxo_hashmap: HashMap::new(),
        }
    }

    /// Insert the inputs of a `Transaction` with the `Block` id
    ///
    /// * `tx` - `Transaction` which the inputs are inserted into `HashMap`
    /// * `block_id` - `Block` id used as value
    pub fn spend_transaction(&mut self, _tx: &Transaction, _block_id: u64) {}

    /// Remove the inputs of a `Transaction` with the `Block` id
    ///
    /// * `tx` - `Transaction` where inputs are inserted, and outputs are removed
    pub fn unspend_transaction(&mut self, _tx: &Transaction) {}

    /// Return the `Block` id based on `Slip`
    ///
    /// * `slip` - `&Slip` as key
    pub fn slip_block_id(&self, slip: &Slip) -> Option<&i64> {
        self.utxo_hashmap.get(slip)
    }
}

#[cfg(test)]
mod test {
    //
    // use super::*;
    // use crate::{
    //     keypair::Keypair,
    //     slip::{Slip, SlipBroadcastType},
    //     transaction::{Transaction, TransactionType},
    // };
    // use std::collections::HashMap;
    //
    // #[test]
    // fn shashmap_test() {
    //     let shashmap = Shashmap::new();
    //     assert_eq!(shashmap.utxo_hashmap, HashMap::new());
    // }
    //
    // #[test]
    // fn shashmap_insert_test() {
    //     let mut shashmap = Shashmap::new();
    //     let keypair = Keypair::new();
    //     let slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
    //     shashmap.insert(slip, 0);
    //     assert!(shashmap.utxo_hashmap.contains_key(&slip));
    // }
    //
    // #[test]
    // fn shashmap_insert_new_transaction_test() {
    //     let mut shashmap = Shashmap::new();
    //
    //     let public_key: PublicKey = PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072").unwrap();
    //     let mut tx_body = Transaction::new(TransactionType::Normal, public_key);
    //     let mut tx Transaction::add_signature(tx_body, &Signature::from_compact(&[0; 64]).unwrap());
    //
    //     let keypair = Keypair::new();
    //     let output_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
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
    //     let mut shashmap = Shashmap::new();
    //     let public_key: PublicKey = PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072").unwrap();
    //     let mut tx_body = Transaction::new(TransactionType::Normal, public_key);
    //     let mut tx Transaction::add_signature(tx_body, &Signature::from_compact(&[0; 64]).unwrap());
    //     let keypair = Keypair::new();
    //     let input_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
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
    //     let mut shashmap = Shashmap::new();
    //     let public_key: PublicKey = PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072").unwrap();
    //     let mut tx_body = Transaction::new(TransactionType::Normal, public_key);
    //     let mut tx Transaction::add_signature(tx_body, &Signature::from_compact(&[0; 64]).unwrap());
    //
    //     let keypair = Keypair::new();
    //     let input_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
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
    //     let mut shashmap = Shashmap::new();
    //
    //     let keypair = Keypair::new();
    //     let input_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
    //
    //     shashmap.spend_slip(&input_slip, 0);
    //
    //     assert!(shashmap.utxo_hashmap.contains_key(&input_slip));
    //     assert_eq!(shashmap.utxo_hashmap.get(&input_slip).unwrap(), &0);
    // }
    //
    // #[test]
    // fn shashmap_unspend_slip_test() {
    //     let mut shashmap = Shashmap::new();
    //
    //     let keypair = Keypair::new();
    //     let input_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
    //
    //     shashmap.unspend_slip(&input_slip);
    //
    //     assert!(shashmap.utxo_hashmap.contains_key(&input_slip));
    //     assert_eq!(shashmap.utxo_hashmap.get(&input_slip).unwrap(), &-1);
    // }
    //
    // #[test]
    // fn shashmap_slip_block_id_test() {
    //     let mut shashmap = Shashmap::new();
    //
    //     let keypair = Keypair::new();
    //     let slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
    //     shashmap.insert(slip, 1);
    //
    //     match shashmap.slip_block_id(&slip) {
    //         Some(id) => assert_eq!(id, &1),
    //         _ => assert!(false),
    //     }
    // }
}
