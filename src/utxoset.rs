use crate::slip::Slip;
use crate::transaction::Transaction;
use std::collections::HashMap;

/// A hashmap storing the byte arrays of `Slip`s as keys
/// with the `Block` ids as values. This is used to enforce when
/// `Slip`s have been spent in the network
#[derive(Clone)]
pub struct UTXOSet {
    utxo_hashmap: HashMap<Vec<u8>, i64>,
}

impl UTXOSet {
    /// Create new `UTXOSet`
    pub fn new() -> Self {
        UTXOSet {
            utxo_hashmap: HashMap::new(),
        }
    }

    /// Insert serizialized slip into UTXO hashmap
    pub fn insert(&mut self, _x: Vec<u8>, _y: u64) {
        self.utxo_hashmap.insert(_x, _y as i64);
    }

    /// Insert serizialized slip into UTXO hashmap
    pub fn insert_new_transaction(&mut self, tx: &Transaction) {
        for to in tx.outputs().iter() {
            self.utxo_hashmap.insert(to.signature_source(), -1);
        }
    }

    /// Insert the inputs of a `Transaction` with the `Block` id
    pub fn spend_transaction(&mut self, tx: &Transaction, _bid: u64) {
        for from in tx.inputs().iter() {
            self.utxo_hashmap
                .insert(from.signature_source(), _bid as i64);
        }
    }

    /// Remove the inputs of a `Transaction` with the `Block` id
    pub fn unspend_transaction(&mut self, tx: &Transaction) {
        for from in tx.inputs().iter() {
            self.utxo_hashmap.insert(from.signature_source(), -1);
        }

        for to in tx.outputs().iter() {
            self.utxo_hashmap.remove(&to.signature_source());
        }
    }

    /// Insert a `Slip`s byte array with the `Block` id
    pub fn spend_slip(&mut self, slip: &Slip, _bid: u64) {
        self.utxo_hashmap
            .insert(slip.signature_source(), _bid as i64);
    }

    /// Insert a `Slip`s byte array with the `Block` id
    pub fn unspend_slip(&mut self, slip: &Slip, _bid: u64) {
        self.utxo_hashmap.insert(slip.signature_source(), -1);
    }

    /// Return the `Block` id based on a `Slip` byte array
    pub fn slip_block_id(&self, slip_index: &Vec<u8>) -> Option<&i64> {
        self.utxo_hashmap.get(slip_index)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn utxoset_test() {
        let utxoset = UTXOSet::new();
        assert_eq!(utxoset.utxo_hashmap, HashMap::new());
    }

    #[test]
    fn utxoset_insert_test() {
        let mut utxoset = UTXOSet::new();
        let byte_array = vec![255, 255, 0, 255];
        utxoset.insert(byte_array.clone(), 1);
        assert!(utxoset.utxo_hashmap.contains_key(&byte_array));
    }

    #[test]
    fn utxoset_insert_new_transaction_test() {
        assert!(false);
    }

    #[test]
    fn utxoset_spend_transaction_test() {
        assert!(false);
    }

    #[test]
    fn utxoset_unspend_transaction_test() {
        assert!(false);
    }

    #[test]
    fn utxoset_spend_slip_test() {
        assert!(false);
    }

    #[test]
    fn utxoset_unspend_slip_test() {
        assert!(false);
    }

    #[test]
    fn utxoset_slip_block_id_test() {
        let mut utxoset = UTXOSet::new();
        let byte_array = vec![255, 255, 0, 255];
        utxoset.insert(byte_array.clone(), 1);

        match utxoset.slip_block_id(&byte_array) {
            Some(id) => assert_eq!(id, &1),
            _ => assert!(false),
        }
    }
}
