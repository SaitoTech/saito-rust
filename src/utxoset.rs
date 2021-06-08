use crate::block::Block;
use crate::blockchain::ForkTuple;
use crate::slip::{OutputSlip, SlipID};
use crate::transaction::Transaction;
use secp256k1::PublicKey;
use std::collections::HashMap;
use std::str::FromStr;

/// TODO document this
#[derive(Debug, Clone)]
enum LongestChainSpentStatus {
    Unspent(u64), // block id
    Spent(u64),   // block id
}

/// TODO document this
#[derive(Debug, Clone)]
enum ForkSpentStatus {
    ForkUnspent([u8; 32]),
    ForkSpent([u8; 32]),
}

/// TODO document this
#[derive(Debug, Clone)]
struct SlipSpentStatus {
    output_slip: OutputSlip,
    longest_chain_status: Option<LongestChainSpentStatus>,
    fork_status: Vec<ForkSpentStatus>,
}

impl SlipSpentStatus {
    pub fn new_on_longest_chain(
        output_slip: OutputSlip,
        longest_chain_status: LongestChainSpentStatus,
    ) -> Self {
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
            .for_each(|tx| self.roll_back_transaction_on_fork(tx, block));
    }

    pub fn roll_forward_on_potential_fork(&mut self, block: &Block) {
        // spendoutputs and spend the inputs
        block
            .transactions()
            .iter()
            .for_each(|tx| self.roll_forward_transaction_on_fork(tx, block));
    }

    pub fn roll_back(&mut self, block: &Block) {
        // unspend outputs and spend the inputs
        block
            .transactions()
            .iter()
            .for_each(|tx| self.roll_back_transaction(tx, block));
    }

    pub fn roll_forward(&mut self, block: &Block) {
        block
            .transactions()
            .iter()
            .for_each(|tx| self.roll_forward_transaction(tx, block));
    }

    /// Loop through the inputs and outputs in a transaction update the hashmap appropriately.
    /// Inputs should be marked back to Unspent, Outputs should have all status set to None. We
    /// do not delete Outputs from the hashmap because they will soon be "unspent" again when
    /// the transaction is rolled forward in another block.
    fn roll_back_transaction(&mut self, _tx: &Transaction, _block: &Block) {}
    /// Loop through the inputs and outputs in a transaction update the hashmap appropriately.
    /// Inputs can just be removed(delete the appropriate ForkSpent from the vector, the
    /// ForkUnspent is still in the vector). Outputs should also have their ForkSpent removed from
    /// the vector.
    fn roll_back_transaction_on_fork(&mut self, _tx: &Transaction, _block: &Block) {}
    /// Loop through the inputs and outputs in a transaction update the hashmap appropriately.
    /// Outputs should be added or marked Unspent, Inputs should be marked Spent. This method
    /// Can be called during a normal new block or during a reorg, so Unspent Outputs may already
    /// be present if we're doing a reorg.
    fn roll_forward_transaction(&mut self, tx: &Transaction, block: &Block) {
        // loop through outputs and mark as unspent. Add them if they aren't already in the
        // hashmap, otherwise update them appropriately
        tx.core
            .outputs()
            .iter()
            .enumerate()
            .for_each(|(index, output)| {
                let slip_id = SlipID::new(tx.core.hash(), index as u64);
                self.shashmap
                    .entry(slip_id)
                    .and_modify(|slip_spent_status| {
                        slip_spent_status.longest_chain_status =
                            Some(LongestChainSpentStatus::Unspent(block.id()));
                    })
                    .or_insert(SlipSpentStatus::new_on_longest_chain(
                        output.clone(),
                        LongestChainSpentStatus::Unspent(block.id()),
                    ));
            });
        // loop through inputs and mark them as Spent, if they're not in the hashmap something is
        // horribly wrong
        tx.core.inputs().iter().for_each(|slip_id| {
            self.shashmap
                .entry(*slip_id)
                .and_modify(|slip_spent_status: &mut SlipSpentStatus| {
                    slip_spent_status.longest_chain_status =
                        Some(LongestChainSpentStatus::Spent(block.id()));
                });
        });
    }
    /// Loop through the inputs and outputs in a transaction update the hashmap appropriately.
    /// Outputs should be added or marked as ForkUnspent, Inputs should be marked ForkSpent.
    /// This method be called when the block is first seen but should never need to be called
    /// during a reorg.
    fn roll_forward_transaction_on_fork(&mut self, tx: &Transaction, block: &Block) {
        tx.core
            .outputs()
            .iter()
            .enumerate()
            .for_each(|(index, output)| {
                let slip_id = SlipID::new(tx.core.hash(), index as u64);
                self.shashmap
                    .entry(slip_id)
                    .and_modify(|slip_spent_status: &mut SlipSpentStatus| {
                        slip_spent_status
                            .fork_status
                            .push(ForkSpentStatus::ForkUnspent(block.hash().clone()));
                    })
                    .or_insert(SlipSpentStatus::new_on_fork(
                        output.clone(),
                        ForkSpentStatus::ForkUnspent(block.hash().clone()),
                    ));
            });
        // loop through inputs and mark them as ForkSpent
        tx.core.inputs().iter().for_each(|slip_id| {
            self.shashmap
                .entry(*slip_id)
                .and_modify(|slip_spent_status: &mut SlipSpentStatus| {
                    slip_spent_status
                        .fork_status
                        .push(ForkSpentStatus::ForkSpent(block.hash().clone()));
                });
        });
    }

    /// Returns true if the slip is Unspent(present in the hashmap and marked Unspent before the
    /// block). The ForkTuple allows us to check for Unspent/Spent status along the fork's
    /// potential new chain more quickly. This can be further optimized in the future.
    pub fn is_slip_spent_at_block(
        &self,
        _slip_id: &SlipID,
        _block: &Block,
        _fork_tuple: &ForkTuple,
    ) -> bool {
        // if the block is on the longest chain
        //   if the Slip is marked Unspent, return false(unspent)
        //   else return true(Slip is considered Spent if it is marked Spent(...) or if it's not in the set)
        // else(the block is in a fork)
        //   if the Slip is marked Unspent
        //      loop through the fork_tuple's new_chain and loop through the forkspentstatus to make sure it wasn't marked as spent somewhere in this fork, return the answer
        //   else if the Slip is marked Spent, but the block_id where it was spend is < the ancestor in the fork
        //      loop through the fork_tuple's new_chain and loop through the forkspentstatus to make sure it wasn't also marked as spent somewhere in this fork, return the answer
        false
    }

    /// Loops through all the SlipIDs(inputs) and return the amount. This is used to validate
    /// that a transaction is balanced.
    pub fn get_total_for_slips(&self, _slip_ids: Vec<SlipID>) -> u64 {
        100
    }

    /// This verifies that the corresponding outputs for the given inputs were all received by
    /// a single address, and, if so, returns that address, otherwise returns None. This is used
    /// to validate that the signer of a transaction is the receiver of all the outputs which
    /// he/she is trying to spend as inputs in a transaction.
    pub fn get_receiver_for_slips(&self, slip_ids: &Vec<SlipID>) -> Option<&PublicKey> {
        if slip_ids.is_empty() {
            None
        } else {
            if let Some(outputs) = slip_ids
                .iter()
                .map(|input| self.output_slip_from_slip_id(input))
                .collect::<Option<Vec<&OutputSlip>>>()
            {
                let first = outputs[0];
                if outputs
                    .iter()
                    .all(|&output| output.address() == first.address())
                {
                    Some(first.address())
                } else {
                    None
                }
            } else {
                None
            }
        }
    }

    /// This is used to get the Output(`OutputSlip`) which corresponds to a given Input(`SlipID`)
    pub fn output_slip_from_slip_id(&self, slip_id: &SlipID) -> Option<&OutputSlip> {
        match self.shashmap.get(slip_id) {
            Some(slip_output) => Some(&slip_output.output_slip),
            None => None,
        }
    }

    pub fn fees_in_transactions(&self, transactions: &Vec<Transaction>) -> u64 {
        let mut input_amt = 0;
        let mut output_amt = 0;

        transactions.iter().for_each(|tx| {
            let input_tot: u64 = tx
                .core
                .inputs()
                .iter()
                .map(|input| self.output_slip_from_slip_id(input).unwrap().amount())
                .sum();
            input_amt = input_amt + input_tot;

            let output_tot: u64 = tx.core.outputs().iter().map(|output| output.amount()).sum();
            output_amt = output_amt + output_tot;
        });

        input_amt - output_amt
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
    //     shashmap.roll_forward_transaction(&tx);
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
