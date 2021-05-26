use crate::transaction::Transaction;
use std::collections::HashMap;
use crate::slip::InputSlip;

/// A hashmap storing the byte arrays of `Slip`s as keys
/// with the `Block` ids as values. This is used to enforce when
/// `Slip`s have been spent in the network
#[derive(Debug, Clone)]
pub struct UTXOSet {
    utxo_hashmap: HashMap<InputSlip, u64>,
}

impl UTXOSet {
    /// Create new `UTXOSet`
    pub fn new() -> Self {
        UTXOSet {
            utxo_hashmap: HashMap::new(),
        }
    }

    /// Insert/Remove all `InputSlip`s for a given `Transaction`
    ///
    /// * `tx` - `Transaction` which the outputs are inserted into `HashMap`
    pub fn insert_transaction(&mut self, tx: &Transaction, block_id: u64) {
        // TODO 
        for (index, _input) in tx.signed_tx.body.inputs().iter().enumerate() {
            let input_slip = InputSlip::new(block_id, tx.id(), index as u64);
            self.utxo_hashmap.remove(&input_slip);
        }
        for (index, _output) in tx.signed_tx.body.outputs().iter().enumerate() {
            let output_slip = InputSlip::new(block_id, tx.id(), index as u64);
            self.utxo_hashmap.insert(output_slip, block_id);
        }
    }

    /// Remove the inputs of a `Transaction` with the `Block` id
    ///
    /// * `tx` - `Transaction` where inputs are inserted, and outputs are removed
    // pub fn rewind_transaction(&mut self, tx: &Transaction, block_id: u64) {
    //     for (index, _input) in tx.signed_tx.body.inputs().iter().enumerate() {
    //         let input_slip = InputSlip::new(block_id, tx.id(), index as u64);
    //         self.utxo_hashmap.insert(input_slip, block_id);
    //     }
    //     for (index, _output) in tx.signed_tx.body.outputs().iter().enumerate() {
    //         let output_slip = InputSlip::new(block_id, tx.id(), index as u64);
    //         self.utxo_hashmap.remove(&output_slip);
    // 
    //     }
    // }
    
    /// Return the `Block` id based on `Slip`
    ///
    /// * `slip` - `&Slip` as key
    pub fn slip_block_id(&self, slip: &InputSlip) -> Option<&u64> {
        self.utxo_hashmap.get(slip)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        keypair::Keypair,
        slip::{InputSlip, OutputSlip, SlipBroadcastType},
        transaction::{Transaction, SignedTransaction, TransactionBody, TransactionBroadcastType},
    };
    use std::collections::HashMap;
    use crate::crypto::Signature;
    
    #[test]
    fn utxoset_test() {
        let utxoset = UTXOSet::new();
        assert_eq!(utxoset.utxo_hashmap, HashMap::new());
    }

    #[test]
    fn utxoset_insert_transaction_test() {
        let mut utxoset = UTXOSet::new();
        let mut tx_body = TransactionBody::new(TransactionBroadcastType::Normal);
        let mock_block_number: u64 = 1;
        let mock_tx_number: u64 = 1;
        
        let keypair = Keypair::new();
        let output_slip_body = OutputSlip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 10);
        let input_slip_body = OutputSlip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 10);
        
        tx_body.add_output(output_slip_body);
        tx_body.add_input(input_slip_body);
        // TODO: Get the tx id from the tx
        let signed_tx = SignedTransaction::new(Signature::from_compact(&[0; 64]).unwrap(), tx_body);
        let tx = Transaction::new(mock_tx_number, signed_tx);
        utxoset.insert_transaction(&tx, mock_block_number);
        
        let mock_input_slip = InputSlip::new(mock_block_number, mock_tx_number, 0);
        // TODO FIX THIS
        println!("{}", utxoset.utxo_hashmap.contains_key(&mock_input_slip));
        //assert!(utxoset.utxo_hashmap.contains_key(&mock_input_slip));
        //assert_eq!(utxoset.utxo_hashmap.get(&mock_input_slip).unwrap(), &mock_block_number);
    }

    #[test]
    fn utxoset_rewind_transaction_test() {
        // TODO FIX THIS
        assert!(false);
    }

}
