use crate::slip::Slip;
use crate::transaction::Transaction;
use std::collections::HashMap;

/// A hashmap storing the byte arrays of `Slip`s as keys
/// with the `Block` ids as values. This is used to enforce when
/// `Slip`s have been spent in the network
#[derive(Clone)]
pub struct UTXOSet {
    utxo_hashmap: HashMap<Slip, i64>,
}

impl UTXOSet {
    /// Create new `UTXOSet`
    pub fn new() -> Self {
        UTXOSet {
            utxo_hashmap: HashMap::new(),
        }
    }

    /// Insert serizialized slip into UTXO hashmap
    ///
    /// * `slip` - `Slip` as our key
    /// * `id` - `Block` id
    pub fn insert(&mut self, slip: Slip, id: u64) {
        self.utxo_hashmap.insert(slip, id as i64);
    }

    /// Insert serizialized slip into UTXO hashmap
    ///
    /// * `tx` - `Transaction` which the outputs are inserted into `HashMap`
    pub fn insert_new_transaction(&mut self, tx: &Transaction) {
        for output in tx.outputs().iter() {
            self.utxo_hashmap.insert(output.clone(), -1);
        }
    }

    /// Insert the inputs of a `Transaction` with the `Block` id
    ///
    /// * `tx` - `Transaction` which the inputs are inserted into `HashMap`
    /// * `block_id` - `Block` id used as value
    pub fn spend_transaction(&mut self, tx: &Transaction, block_id: u64) {
        for input in tx.inputs().iter() {
            self.utxo_hashmap.insert(input.clone(), block_id as i64);
        }
    }

    /// Remove the inputs of a `Transaction` with the `Block` id
    ///
    /// * `tx` - `Transaction` where inputs are inserted, and outputs are removed
    pub fn unspend_transaction(&mut self, tx: &Transaction) {
        for input in tx.inputs().iter() {
            self.utxo_hashmap.insert(input.clone(), -1);
        }

        for outer in tx.outputs().iter() {
            self.utxo_hashmap.remove(&outer);
        }
    }

    /// Insert a `Slip`s byte array with the `Block` id
    ///
    /// * `slip` - `Slip` as key
    /// * `block_id` - `Block` id as value
    pub fn spend_slip(&mut self, slip: &Slip, _bid: u64) {
        self.utxo_hashmap.insert(slip.clone(), _bid as i64);
    }

    /// Insert a `Slip`s byte array with the `Block` id
    ///
    /// * `slip` - `&Slip` as key
    pub fn unspend_slip(&mut self, slip: &Slip) {
        self.utxo_hashmap.insert(slip.clone(), -1);
    }

    /// Return the `Block` id based on `Slip`
    ///
    /// * `slip` - `&Slip` as key
    pub fn slip_block_id(&self, slip: &Slip) -> Option<&i64> {
        self.utxo_hashmap.get(slip)
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
    use std::collections::HashMap;

    #[test]
    fn utxoset_test() {
        let utxoset = UTXOSet::new();
        assert_eq!(utxoset.utxo_hashmap, HashMap::new());
    }

    #[test]
    fn utxoset_insert_test() {
        let mut utxoset = UTXOSet::new();
        let keypair = Keypair::new();
        let slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
        utxoset.insert(slip, 0);
        assert!(utxoset.utxo_hashmap.contains_key(&slip));
    }

    #[test]
    fn utxoset_insert_new_transaction_test() {
        let mut utxoset = UTXOSet::new();
        let mut tx = Transaction::new(TransactionBroadcastType::Normal);

        let keypair = Keypair::new();
        let output_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);

        tx.add_output(output_slip);

        utxoset.insert_new_transaction(&tx);

        assert!(utxoset.utxo_hashmap.contains_key(&output_slip));
        assert_eq!(utxoset.utxo_hashmap.get(&output_slip).unwrap(), &-1);
    }

    #[test]
    fn utxoset_spend_transaction_test() {
        let mut utxoset = UTXOSet::new();
        let mut tx = Transaction::new(TransactionBroadcastType::Normal);

        let keypair = Keypair::new();
        let input_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);

        tx.add_input(input_slip);

        utxoset.spend_transaction(&tx, 0);

        assert!(utxoset.utxo_hashmap.contains_key(&input_slip));
        assert_eq!(utxoset.utxo_hashmap.get(&input_slip).unwrap(), &0);
    }

    #[test]
    fn utxoset_unspend_transaction_test() {
        let mut utxoset = UTXOSet::new();
        let mut tx = Transaction::new(TransactionBroadcastType::Normal);

        let keypair = Keypair::new();
        let input_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);

        tx.add_input(input_slip);

        utxoset.unspend_transaction(&tx);

        assert!(utxoset.utxo_hashmap.contains_key(&input_slip));
        assert_eq!(utxoset.utxo_hashmap.get(&input_slip).unwrap(), &-1);
    }

    #[test]
    fn utxoset_spend_slip_test() {
        let mut utxoset = UTXOSet::new();

        let keypair = Keypair::new();
        let input_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);

        utxoset.spend_slip(&input_slip, 0);

        assert!(utxoset.utxo_hashmap.contains_key(&input_slip));
        assert_eq!(utxoset.utxo_hashmap.get(&input_slip).unwrap(), &0);
    }

    #[test]
    fn utxoset_unspend_slip_test() {
        let mut utxoset = UTXOSet::new();

        let keypair = Keypair::new();
        let input_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);

        utxoset.unspend_slip(&input_slip);

        assert!(utxoset.utxo_hashmap.contains_key(&input_slip));
        assert_eq!(utxoset.utxo_hashmap.get(&input_slip).unwrap(), &-1);
    }

    #[test]
    fn utxoset_slip_block_id_test() {
        let mut utxoset = UTXOSet::new();

        let keypair = Keypair::new();
        let slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
        utxoset.insert(slip, 1);

        match utxoset.slip_block_id(&slip) {
            Some(id) => assert_eq!(id, &1),
            _ => assert!(false),
        }
    }
}
