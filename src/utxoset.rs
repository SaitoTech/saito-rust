use crate::slip::SlipReference;
use crate::transaction::RuntimeTransaction;
use std::collections::HashMap;

/// A hashmap storing the byte arrays of `Slip`s as keys
/// with the `Block` ids as values. This is used to enforce when
/// `Slip`s have been spent in the network
#[derive(Debug, Clone)]
pub struct UTXOSet {
    utxo_hashmap: HashMap<SlipReference, u64>,
}

impl UTXOSet {
    /// Create new `UTXOSet`
    pub fn new() -> Self {
        UTXOSet {
            utxo_hashmap: HashMap::new(),
        }
    }

    /// Insert/Remove all `SlipReference`s for a given `RuntimeTransaction`
    ///
    /// * `tx` - `RuntimeTransaction` which the outputs are inserted into `HashMap`
    pub fn insert_transaction(&mut self, tx: &RuntimeTransaction, block_id: u64) {
        for input_slip in tx.signed_tx.body.inputs().iter() {
            self.utxo_hashmap.remove(&input_slip);
        }
        for (index, _output_slip) in tx.signed_tx.body.outputs().iter().enumerate() {
            let output_as_input_slip = SlipReference::new(block_id, tx.id(), index as u64);
            self.utxo_hashmap.insert(output_as_input_slip, block_id);
        }
    }

    /// Remove the inputs of a `RuntimeTransaction` with the `Block` id
    ///
    /// * `tx` - `RuntimeTransaction` where inputs are inserted, and outputs are removed
    pub fn rewind_transaction(&mut self, tx: &RuntimeTransaction, block_id: u64) {
        for input_slip in tx.signed_tx.body.inputs().iter() {
            self.utxo_hashmap.insert(*input_slip, block_id);
        }
        for (index, _output) in tx.signed_tx.body.outputs().iter().enumerate() {
            let output_as_input_slip = SlipReference::new(block_id, tx.id(), index as u64);
            self.utxo_hashmap.remove(&output_as_input_slip);
        }
    }

    /// Return the `Block` id based on `Slip`
    ///
    /// * `slip` - `&Slip` as key
    pub fn slip_block_id(&self, slip: &SlipReference) -> Option<&u64> {
        self.utxo_hashmap.get(slip)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::crypto::Signature;
    use crate::{
        keypair::Keypair,
        slip::{SaitoSlip, SlipBroadcastType, SlipReference},
        transaction::{
            RuntimeTransaction, SignedTransaction, TransactionBody, TransactionBroadcastType,
        },
    };
    use std::collections::HashMap;

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
        let output_slip =
            SaitoSlip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 10);
        let input_slip = SlipReference::new(mock_block_number, mock_tx_number, 0);

        tx_body.add_output(output_slip);
        tx_body.add_input(input_slip);
        // TODO: Get the tx id from the tx
        let signed_tx = SignedTransaction::new(Signature::from_compact(&[0; 64]).unwrap(), tx_body);
        let tx = RuntimeTransaction::new(mock_tx_number, signed_tx);
        utxoset.insert_transaction(&tx, mock_block_number);

        let mock_input_slip = SlipReference::new(mock_block_number, mock_tx_number, 0);
        assert!(utxoset.utxo_hashmap.contains_key(&mock_input_slip));
        assert_eq!(
            utxoset.utxo_hashmap.get(&mock_input_slip).unwrap(),
            &mock_block_number
        );
    }

    #[test]
    fn utxoset_rewind_transaction_test() {
        // TODO FIX THIS
        //assert!(false);
    }
}
