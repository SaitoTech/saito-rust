use crate::slip::Slip;
use crate::transaction::Transaction;
use std::collections::HashMap;

/// A hashmap storing the byte arrays of `Slip`s as keys
/// with the `Block` ids as values. This is used to enforce when
/// `Slip`s have been spent in the network
#[derive(Clone)]
pub struct UTXOSet {
    utxo_hashmap: HashMap<[u8; 32], i64>,
}

impl UTXOSet {
    /// Create new `UTXOSet`
    pub fn new() -> Self {
        UTXOSet {
            utxo_hashmap: HashMap::new(),
        }
    }

    /// Insert serizialized slip into UTXO hashmap
    pub fn insert(&mut self, _x: [u8; 32], _y: u64) {
        self.utxo_hashmap.insert(_x, _y as i64);
    }

    /// Insert serizialized slip into UTXO hashmap
    pub fn insert_new_transaction(&mut self, tx: &Transaction) {
        for output in tx.outputs().iter() {
            self.utxo_hashmap.insert(output.hash(), -1);
        }
    }

    /// Insert the inputs of a `Transaction` with the `Block` id
    pub fn spend_transaction(&mut self, tx: &Transaction, _bid: u64) {
        for input in tx.inputs().iter() {
            self.utxo_hashmap.insert(input.hash(), _bid as i64);
        }
    }

    /// Remove the inputs of a `Transaction` with the `Block` id
    pub fn unspend_transaction(&mut self, tx: &Transaction) {
        for input in tx.inputs().iter() {
            self.utxo_hashmap.insert(input.hash(), -1);
        }

        for outer in tx.outputs().iter() {
            self.utxo_hashmap.remove(&outer.hash());
        }
    }

    /// Insert a `Slip`s byte array with the `Block` id
    pub fn spend_slip(&mut self, slip: &Slip, _bid: u64) {
        self.utxo_hashmap.insert(slip.hash(), _bid as i64);
    }

    /// Insert a `Slip`s byte array with the `Block` id
    pub fn unspend_slip(&mut self, slip: &Slip) {
        self.utxo_hashmap.insert(slip.hash(), -1);
    }

    /// Return the `Block` id based on a `Slip` byte array
    pub fn slip_block_id(&self, slip_index: &[u8; 32]) -> Option<&i64> {
        self.utxo_hashmap.get(slip_index)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{
        keypair::Keypair,
        slip::{Slip, SlipBroadcastType},
        transaction::{Transaction, TransactionBroadcastType},
    };
    use std::{collections::HashMap, convert::TryInto};

    #[test]
    fn utxoset_test() {
        let utxoset = UTXOSet::new();
        assert_eq!(utxoset.utxo_hashmap, HashMap::new());
    }

    #[test]
    fn utxoset_insert_test() {
        let mut utxoset = UTXOSet::new();
        let byte_array: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
        let hash: [u8; 32] = byte_array.as_slice().try_into().unwrap();
        utxoset.insert(hash.clone(), 1);
        assert!(utxoset.utxo_hashmap.contains_key(&hash));
    }

    #[test]
    fn utxoset_insert_new_transaction_test() {
        let mut utxoset = UTXOSet::new();
        let mut tx = Transaction::new(TransactionBroadcastType::Normal);

        let keypair = Keypair::new();
        let output_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);

        tx.add_output(output_slip);

        utxoset.insert_new_transaction(&tx);

        assert!(utxoset.utxo_hashmap.contains_key(&output_slip.hash()));
        assert_eq!(utxoset.utxo_hashmap.get(&output_slip.hash()).unwrap(), &-1);
    }

    #[test]
    fn utxoset_spend_transaction_test() {
        let mut utxoset = UTXOSet::new();
        let mut tx = Transaction::new(TransactionBroadcastType::Normal);

        let keypair = Keypair::new();
        let input_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);

        tx.add_input(input_slip);

        utxoset.spend_transaction(&tx, 0);

        assert!(utxoset.utxo_hashmap.contains_key(&input_slip.hash()));
        assert_eq!(utxoset.utxo_hashmap.get(&input_slip.hash()).unwrap(), &0);
    }

    #[test]
    fn utxoset_unspend_transaction_test() {
        let mut utxoset = UTXOSet::new();
        let mut tx = Transaction::new(TransactionBroadcastType::Normal);

        let keypair = Keypair::new();
        let input_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);

        tx.add_input(input_slip);

        utxoset.unspend_transaction(&tx);

        assert!(utxoset.utxo_hashmap.contains_key(&input_slip.hash()));
        assert_eq!(utxoset.utxo_hashmap.get(&input_slip.hash()).unwrap(), &-1);
    }

    #[test]
    fn utxoset_spend_slip_test() {
        let mut utxoset = UTXOSet::new();

        let keypair = Keypair::new();
        let input_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);

        utxoset.spend_slip(&input_slip, 0);

        assert!(utxoset.utxo_hashmap.contains_key(&input_slip.hash()));
        assert_eq!(utxoset.utxo_hashmap.get(&input_slip.hash()).unwrap(), &0);
    }

    #[test]
    fn utxoset_unspend_slip_test() {
        let mut utxoset = UTXOSet::new();

        let keypair = Keypair::new();
        let input_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);

        utxoset.unspend_slip(&input_slip);

        assert!(utxoset.utxo_hashmap.contains_key(&input_slip.hash()));
        assert_eq!(utxoset.utxo_hashmap.get(&input_slip.hash()).unwrap(), &-1);
    }

    #[test]
    fn utxoset_slip_block_id_test() {
        let mut utxoset = UTXOSet::new();
        let byte_array: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
        let hash: [u8; 32] = byte_array.as_slice().try_into().unwrap();

        utxoset.insert(hash.clone(), 1);

        match utxoset.slip_block_id(&hash) {
            Some(id) => assert_eq!(id, &1),
            _ => assert!(false),
        }
    }
}
