use crate::block::Block;
use crate::blockchain::ForkChains;
use crate::crypto::Sha256Hash;
use crate::slip::{OutputSlip, SlipID};
use crate::transaction::{Transaction, TransactionCore};
use secp256k1::PublicKey;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
enum LongestChainSpentTime {
    BeforeUnspent,
    BetweenUnspentAndSpent,
    AfterSpent,
    NeverExisted,
    AfterSpentOrNeverExisted,
}

/// TODO document this
#[derive(Debug, Clone, Hash, PartialEq)]
enum ForkSpentStatus {
    ForkUnspent,
    ForkSpent,
}

/// TODO document this
#[derive(Debug, Clone, PartialEq)]
struct SlipSpentStatus {
    output_slip: OutputSlip,
    //longest_chain_status: Option<LongestChainSpentStatus>,
    longest_chain_unspent_block_id: Option<u64>,
    longest_chain_spent_block_id: Option<u64>,
    fork_status: HashMap<Sha256Hash, ForkSpentStatus>,
}
impl SlipSpentStatus {
    pub fn new_on_longest_chain(output_slip: OutputSlip, unspent_block_id: u64) -> Self {
        SlipSpentStatus {
            output_slip: output_slip,
            longest_chain_unspent_block_id: Some(unspent_block_id),
            longest_chain_spent_block_id: None,
            fork_status: HashMap::new(),
        }
    }

    pub fn new_on_fork(output_slip: OutputSlip, block_hash: Sha256Hash) -> Self {
        let mut fork_status_map: HashMap<Sha256Hash, ForkSpentStatus> = HashMap::new();
        fork_status_map.insert(block_hash, ForkSpentStatus::ForkUnspent);
        SlipSpentStatus {
            output_slip: output_slip,
            longest_chain_unspent_block_id: None,
            longest_chain_spent_block_id: None,
            fork_status: fork_status_map,
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

    pub fn roll_back_on_fork(&mut self, block: &Block) {
        block
            .transactions()
            .iter()
            .for_each(|tx| self.roll_back_transaction_on_fork(tx, block));
    }

    pub fn roll_forward_on_fork(&mut self, block: &Block) {
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
    fn roll_back_transaction(&mut self, tx: &Transaction, _block: &Block) {
        tx.core
            .outputs()
            .iter()
            .enumerate()
            .for_each(|(index, _output)| {
                let slip_id = SlipID::new(tx.hash(), index as u64);
                let entry = self
                    .shashmap
                    .entry(slip_id)
                    .and_modify(|slip_spent_status| {
                        slip_spent_status.longest_chain_unspent_block_id = None;
                    });
                if let Entry::Vacant(_o) = entry {
                    panic!("Output status not found in hashmap!");
                }
            });
        tx.core
            .inputs()
            .iter()
            .enumerate()
            .for_each(|(_index, input)| {
                let entry = self.shashmap.entry(*input).and_modify(|slip_spent_status| {
                    slip_spent_status.longest_chain_spent_block_id = None;
                });
                if let Entry::Vacant(_o) = entry {
                    panic!("Input status not found in hashmap!");
                }
            });
    }
    /// Loop through the inputs and outputs in a transaction update the hashmap appropriately.
    /// Inputs can just be removed(delete the appropriate ForkSpent from the vector, the
    /// ForkUnspent is still in the vector). Outputs should also have their ForkSpent removed from
    /// the vector.
    fn roll_back_transaction_on_fork(&mut self, tx: &Transaction, block: &Block) {
        tx.core
            .outputs()
            .iter()
            .enumerate()
            .for_each(|(index, _output)| {
                let slip_id = SlipID::new(tx.hash(), index as u64);
                let entry = self
                    .shashmap
                    .entry(slip_id)
                    .and_modify(|slip_spent_status| {
                        slip_spent_status.fork_status.remove(&block.hash());
                    });
                if let Entry::Vacant(_o) = entry {
                    panic!("Output fork status not found in hashmap!");
                }
            });
        tx.core
            .inputs()
            .iter()
            .enumerate()
            .for_each(|(_index, input)| {
                let entry = self.shashmap.entry(*input).and_modify(|slip_spent_status| {
                    slip_spent_status.fork_status.remove(&block.hash());
                });
                if let Entry::Vacant(_o) = entry {
                    panic!("Input fork status not found in hashmap!");
                }
            });
    }
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
                let slip_id = SlipID::new(tx.hash(), index as u64);
                self.shashmap
                    .entry(slip_id)
                    .and_modify(|slip_spent_status| {
                        slip_spent_status.longest_chain_spent_block_id = Some(block.id());
                    })
                    .or_insert(SlipSpentStatus::new_on_longest_chain(
                        output.clone(),
                        block.id(),
                    ));
            });
        // loop through inputs and mark them as Spent, if they're not in the hashmap something is
        // horribly wrong
        tx.core.inputs().iter().for_each(|slip_id| {
            self.shashmap
                .entry(*slip_id)
                .and_modify(|slip_spent_status: &mut SlipSpentStatus| {
                    slip_spent_status.longest_chain_spent_block_id = Some(block.id());
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
                let slip_id = SlipID::new(tx.hash(), index as u64);
                self.shashmap
                    .entry(slip_id)
                    .and_modify(|slip_spent_status: &mut SlipSpentStatus| {
                        slip_spent_status
                            .fork_status
                            .insert(block.hash(), ForkSpentStatus::ForkUnspent);
                    })
                    .or_insert(SlipSpentStatus::new_on_fork(output.clone(), block.hash()));
            });
        // loop through inputs and mark them as ForkSpent
        tx.core.inputs().iter().for_each(|slip_id| {
            self.shashmap
                .entry(*slip_id)
                .and_modify(|slip_spent_status: &mut SlipSpentStatus| {
                    slip_spent_status
                        .fork_status
                        .insert(block.hash(), ForkSpentStatus::ForkSpent);
                });
        });
    }

    fn longest_chain_spent_time_status(
        &self,
        slip_id: &SlipID,
        block_id: u64,
    ) -> LongestChainSpentTime {
        match &self.shashmap.get(slip_id) {
            Some(status) => {
                match status.longest_chain_unspent_block_id {
                    Some(longest_chain_unspent_block_id) => {
                        match status.longest_chain_spent_block_id {
                            Some(longest_chain_spent_block_id) => {
                                if longest_chain_unspent_block_id <= block_id
                                    && longest_chain_spent_block_id > block_id
                                {
                                    // There is a spent_block_id but we are interested in the state of the slip before it was spent,
                                    // this is useful when looking at the common_ancestor of forks.
                                    LongestChainSpentTime::BetweenUnspentAndSpent
                                } else if longest_chain_unspent_block_id <= block_id {
                                    // The slip was already spent
                                    LongestChainSpentTime::AfterSpent
                                } else {
                                    // the slip was unspent and spent after this block id
                                    LongestChainSpentTime::NeverExisted
                                }
                            }
                            None => {
                                if longest_chain_unspent_block_id <= block_id {
                                    // The slip is created/unspent before this block and we don't have any spent block id
                                    LongestChainSpentTime::BetweenUnspentAndSpent
                                } else {
                                    // The slip is created/unspent after this block id
                                    LongestChainSpentTime::BeforeUnspent
                                }
                            }
                        }
                    }
                    // The slip is in the utxoset but it's unspent_block_id is set to None, it's been created/unspent but
                    // then set back to None to indicate that the output hasn't been created yet
                    None => LongestChainSpentTime::AfterSpentOrNeverExisted,
                }
            }
            // The slip is not in the utxoset, either it was spent and deleted or it never existed
            None => LongestChainSpentTime::AfterSpentOrNeverExisted,
        }
    }
    /// Returns true if the slip is Unspent(present in the hashmap and marked Unspent before the
    /// block). The ForkTuple allows us to check for Unspent/Spent status along the fork's
    /// potential new chain more quickly. This can be further optimized in the future.
    pub fn is_slip_spendable_at_block(&self, slip_id: &SlipID, block_id: u64) -> bool {
        let longest_chain_spent_time = self.longest_chain_spent_time_status(slip_id, block_id);
        longest_chain_spent_time == LongestChainSpentTime::BetweenUnspentAndSpent
    }
    /// Returns true if the slip is Unspent(present in the hashmap and marked Unspent before the
    /// block). The ForkTuple allows us to check for Unspent/Spent status along the fork's
    /// potential new chain more quickly. This can be further optimized in the future.
    pub fn is_slip_spendable_at_fork_block(
        &self,
        slip_id: &SlipID,
        fork_chains: &ForkChains,
    ) -> bool {
        let longest_chain_spent_time =
            self.longest_chain_spent_time_status(slip_id, fork_chains.ancestor_block.id());
        if longest_chain_spent_time == LongestChainSpentTime::AfterSpent {
            return false;
        }
        let mut return_val = false;
        if longest_chain_spent_time == LongestChainSpentTime::BetweenUnspentAndSpent {
            // it must not be spent in this fork
            fork_chains.new_chain.iter().for_each(|block_hash| {
                match &self.shashmap.get(slip_id) {
                    Some(status) => match status.fork_status.get(block_hash) {
                        Some(fork_spend_status) => {
                            if fork_spend_status == &ForkSpentStatus::ForkSpent {
                                return_val = false;
                            }
                        }
                        None => {
                            return_val = true;
                        }
                    },
                    None => {
                        return_val = true;
                    }
                };
            });
        } else {
            // it must be unspent but not spent in this fork
            let mut is_spent = false;
            let mut is_unspent = false;
            fork_chains.new_chain.iter().for_each(|block_hash| {
                match &self.shashmap.get(slip_id) {
                    Some(status) => match status.fork_status.get(block_hash) {
                        Some(fork_spend_status) => {
                            if fork_spend_status == &ForkSpentStatus::ForkSpent {
                                is_spent = true;
                            } else if fork_spend_status == &ForkSpentStatus::ForkUnspent {
                                is_unspent = true;
                            }
                        }
                        None => {}
                    },
                    None => {
                        is_unspent = false;
                    }
                };
            });
            return_val = is_unspent && !is_spent;
        }
        return_val
    }

    /// Loops through all the SlipIDs(inputs) and return the amount. This is used to validate
    /// that a transaction is balanced.
    ///
    /// If one of the slips is not valid, the function returns 0
    pub fn get_total_for_inputs(&self, slip_ids: Vec<SlipID>) -> Option<u64> {
        if slip_ids.is_empty() {
            None
        } else {
            if let Some(outputs) = slip_ids
                .iter()
                .map(|input| self.output_slip_from_slip_id(input))
                .collect::<Option<Vec<&OutputSlip>>>()
            {
                Some(outputs.iter().map(|output| output.amount()).sum())
            } else {
                None
            }
        }
    }

    /// This verifies that the corresponding outputs for the given inputs were all received by
    /// a single address, and, if so, returns that address, otherwise returns None. This is used
    /// to validate that the signer of a transaction is the receiver of all the outputs which
    /// he/she is trying to spend as inputs in a transaction.
    pub fn get_receiver_for_inputs(&self, slip_ids: &Vec<SlipID>) -> Option<&PublicKey> {
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

    pub fn transaction_fees(&self, tx_core: &TransactionCore) -> u64 {
        let input_amt: u64 = tx_core
            .inputs()
            .iter()
            .map(|input| self.output_slip_from_slip_id(input).unwrap().amount())
            .sum();

        let output_amt: u64 = tx_core.outputs().iter().map(|output| output.amount()).sum();

        input_amt - output_amt
    }

    pub fn transaction_routing_fees(&self, tx: &Transaction) -> u64 {
        let tx_fees = self.transaction_fees(&tx.core);

        let factor;
        let path_len;

        if tx.path().len() > 0 {
            factor = tx.path().len();
            path_len = tx.path().len();
        } else {
            factor = 1;
            path_len = 1;
        }

        ((tx_fees as f64) / path_len.pow(factor as u32) as f64).round() as u64
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{crypto::hash_bytes, keypair::Keypair, test_utilities, transaction::Hop};

    #[tokio::test]
    async fn roll_forward_and_back_test() {
        let keypair = Keypair::new();
        let (mut blockchain, slips) =
            test_utilities::make_mock_blockchain_and_slips(&keypair, 1).await;

        let from_slip = slips.first().unwrap().0;
        let block = test_utilities::make_mock_block(&keypair, [0; 32], 1, from_slip);

        let tx = block.transactions().first().unwrap();
        let slip_id = SlipID::new(tx.hash(), 0);

        blockchain.utxoset.roll_forward(&block);

        // assert that we have a new value in the shashmap to spend
        if let Some(slip_spent_status) = blockchain.utxoset.shashmap.get(&slip_id) {
            assert_eq!(
                slip_spent_status,
                &SlipSpentStatus::new_on_longest_chain(
                    *(&block)
                        .transactions()
                        .first()
                        .unwrap()
                        .core
                        .outputs()
                        .first()
                        .unwrap(),
                    (&block).id()
                )
            );
        }

        if let Some(slip_spent_status) = blockchain.utxoset.shashmap.get(&from_slip) {
            assert_eq!(
                slip_spent_status.longest_chain_spent_block_id,
                Some(block.id())
            )
        }

        blockchain.utxoset.roll_back(&block);

        if let Some(slip_spent_status) = blockchain.utxoset.shashmap.get(&slip_id) {
            assert_eq!(slip_spent_status.longest_chain_unspent_block_id, None);
        }

        if let Some(slip_spent_status) = blockchain.utxoset.shashmap.get(&from_slip) {
            assert_eq!(slip_spent_status.fork_status.get(&block.hash()), None)
        }
    }

    #[tokio::test]
    async fn roll_forward_and_back_fork_test() {
        let keypair = Keypair::new();
        let (mut blockchain, slips) =
            test_utilities::make_mock_blockchain_and_slips(&keypair, 1).await;

        let from_slip = slips.first().unwrap().0;
        let block = test_utilities::make_mock_block(&keypair, [0; 32], 1, from_slip);

        let tx = block.transactions().first().unwrap();
        let slip_id = SlipID::new(tx.hash(), 0);

        blockchain.utxoset.roll_forward_on_fork(&block);

        // assert that we have anew value in the shashmap to have
        if let Some(slip_spent_status) = blockchain.utxoset.shashmap.get(&slip_id) {
            assert_eq!(
                slip_spent_status,
                &SlipSpentStatus::new_on_fork(
                    *(&block)
                        .transactions()
                        .first()
                        .unwrap()
                        .core
                        .outputs()
                        .first()
                        .unwrap(),
                    (&block).hash()
                )
            );
        }

        if let Some(slip_spent_status) = blockchain.utxoset.shashmap.get(&from_slip) {
            assert_eq!(
                slip_spent_status.fork_status.get(&block.hash()),
                Some(&ForkSpentStatus::ForkSpent)
            )
        }

        blockchain.utxoset.roll_back_on_fork(&block);

        if let Some(slip_spent_status) = blockchain.utxoset.shashmap.get(&slip_id) {
            assert_eq!(slip_spent_status.longest_chain_unspent_block_id, None);

            assert_eq!(slip_spent_status.fork_status.get(&block.hash()), None)
        }
    }

    #[tokio::test]
    async fn roll_forward_and_back_transaction_test() {
        let keypair = Keypair::new();
        let (mut blockchain, slips) =
            test_utilities::make_mock_blockchain_and_slips(&keypair, 10).await;
        // make a mock tx
        let (slip_id, output_slip) = slips[0];
        let mock_tx = test_utilities::make_mock_tx(
            slip_id,
            output_slip.amount(),
            keypair.public_key().clone(),
        );
        // get an input for the mock_tx's output
        let outputs_input_a = SlipID::new(mock_tx.hash(), 0);
        // make a mock block with the tx
        let first_block = blockchain.latest_block().unwrap();
        let new_block_a = test_utilities::make_mock_block_with_tx(
            first_block.hash(),
            first_block.id() + 1,
            mock_tx,
        );

        // make a block where we will test spendability
        let new_block_b =
            test_utilities::make_mock_block_empty(first_block.hash(), new_block_a.id() + 1);
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_block(&outputs_input_a, new_block_b.id()));
        // roll_forward block #2
        blockchain
            .utxoset
            .roll_forward_transaction(&new_block_a.transactions()[0], &new_block_a);
        // assert that tx in block #2 is spendable in block #3
        assert!(blockchain
            .utxoset
            .is_slip_spendable_at_block(&outputs_input_a, new_block_b.id()));
        blockchain
            .utxoset
            .roll_back_transaction(&new_block_a.transactions()[0], &new_block_a);
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_block(&outputs_input_a, new_block_b.id()));
    }

    #[tokio::test]
    async fn roll_forward_and_back_transaction_on_fork_test() {
        let keypair = Keypair::new();
        let (mut blockchain, slips) =
            test_utilities::make_mock_blockchain_and_slips(&keypair, 10).await;
        // make a mock tx
        let (slip_id, output_slip) = slips[0];
        let mock_tx_a = test_utilities::make_mock_tx(
            slip_id,
            output_slip.amount(),
            keypair.public_key().clone(),
        );
        // get an input for the mock_tx's output
        let outputs_input_a = SlipID::new(mock_tx_a.hash(), 0);
        let mock_tx_b = test_utilities::make_mock_tx(
            outputs_input_a,
            output_slip.amount(),
            keypair.public_key().clone(),
        );
        let outputs_input_b = SlipID::new(mock_tx_b.hash(), 0);

        // make mock blocks so we can use their hashes
        let first_block = blockchain.latest_block().unwrap();
        let new_block_a = test_utilities::make_mock_block_with_tx(
            first_block.hash(),
            first_block.id() + 1,
            mock_tx_a,
        );
        let new_block_b = test_utilities::make_mock_block_with_tx(
            new_block_a.hash().clone(),
            new_block_a.id().clone() + 1,
            mock_tx_b,
        );

        // roll_forward tx 1 (as it would if block #2 were added), it should be spendable in the next fork block
        blockchain
            .utxoset
            .roll_forward_transaction(&new_block_a.transactions()[0], &new_block_a);
        let fork_chains: ForkChains = ForkChains {
            ancestor_block: new_block_a.clone(),
            old_chain: vec![],
            new_chain: vec![new_block_a.hash(), new_block_b.hash()],
        };
        // a should be spendable, but not b
        assert_eq!(
            blockchain
                .utxoset
                .longest_chain_spent_time_status(&outputs_input_a, new_block_a.id() - 1),
            LongestChainSpentTime::BeforeUnspent
        );
        assert_eq!(
            blockchain
                .utxoset
                .longest_chain_spent_time_status(&outputs_input_a, new_block_a.id()),
            LongestChainSpentTime::BetweenUnspentAndSpent
        );
        assert!(blockchain
            .utxoset
            .is_slip_spendable_at_block(&outputs_input_a, new_block_b.id()));
        assert!(blockchain
            .utxoset
            .is_slip_spendable_at_fork_block(&outputs_input_a, &fork_chains));
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_block(&outputs_input_b, new_block_b.id()));
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_fork_block(&outputs_input_b, &fork_chains));

        // roll_back tx (as it would if block #2 were rolled back), it should no longer be spendable in the next fork block
        blockchain
            .utxoset
            .roll_back_transaction(&new_block_a.transactions()[0], &new_block_a);
        assert_eq!(
            blockchain
                .utxoset
                .longest_chain_spent_time_status(&outputs_input_a, new_block_a.id() - 1),
            LongestChainSpentTime::AfterSpentOrNeverExisted
        );
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_block(&outputs_input_a, new_block_b.id()));
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_fork_block(&outputs_input_a, &fork_chains));
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_block(&outputs_input_b, new_block_b.id()));
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_fork_block(&outputs_input_b, &fork_chains));

        // roll forward tx like it's in a potential fork
        blockchain
            .utxoset
            .roll_forward_transaction_on_fork(&new_block_a.transactions()[0], &new_block_a);
        let fork_chains: ForkChains = ForkChains {
            ancestor_block: new_block_a.clone(),
            old_chain: vec![],
            new_chain: vec![new_block_a.hash(), new_block_b.hash()],
        };
        // it should be spendable at block b as a new fork but not spendable at block id 1
        // on the longest chain
        assert!(blockchain
            .utxoset
            .is_slip_spendable_at_fork_block(&outputs_input_a, &fork_chains));
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_block(&outputs_input_a, new_block_a.id()));
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_block(&outputs_input_b, new_block_a.id()));
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_fork_block(&outputs_input_b, &fork_chains));

        // roll forward a tx that spends the output, it's input should become unspendable again and
        // tx b should become spendable on the fork
        blockchain
            .utxoset
            .roll_forward_transaction_on_fork(&new_block_b.transactions()[0], &new_block_b);
        let fork_chains: ForkChains = ForkChains {
            ancestor_block: new_block_a.clone(),
            old_chain: vec![],
            new_chain: vec![new_block_a.hash(), new_block_b.hash(), [0; 32]],
        };
        assert_eq!(
            blockchain
                .utxoset
                .longest_chain_spent_time_status(&outputs_input_b, new_block_a.id() - 1),
            LongestChainSpentTime::AfterSpentOrNeverExisted
        );
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_block(&outputs_input_a, new_block_a.id()));
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_fork_block(&outputs_input_a, &fork_chains));
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_block(&outputs_input_b, new_block_a.id()));
        assert!(blockchain
            .utxoset
            .is_slip_spendable_at_fork_block(&outputs_input_b, &fork_chains));

        // roll back the tx that spent it, it input should become spendable again
        blockchain
            .utxoset
            .roll_back_transaction_on_fork(&new_block_b.transactions()[0], &new_block_b);
        let fork_chains: ForkChains = ForkChains {
            ancestor_block: new_block_a.clone(),
            old_chain: vec![],
            new_chain: vec![new_block_a.hash(), new_block_b.hash()],
        };
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_block(&outputs_input_a, new_block_a.id()));
        assert!(blockchain
            .utxoset
            .is_slip_spendable_at_fork_block(&outputs_input_a, &fork_chains));
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_block(&outputs_input_b, new_block_a.id()));
        assert!(!blockchain
            .utxoset
            .is_slip_spendable_at_fork_block(&outputs_input_b, &fork_chains));
    }

    #[tokio::test]
    async fn get_total_for_inputs_test() {
        let keypair = Keypair::new();
        let (blockchain, slips) =
            test_utilities::make_mock_blockchain_and_slips(&keypair, 10).await;
        let mut inputs = vec![];
        slips.iter().for_each(|(slip_id, _output_slip)| {
            inputs.push(*slip_id);
        });

        let total = blockchain.utxoset.get_total_for_inputs(inputs);
        assert_eq!(50_000_0000_0000, total.unwrap());
    }

    #[tokio::test]
    async fn get_receiver_for_inputs_test() {
        let keypair = Keypair::new();
        let (blockchain, slips) =
            test_utilities::make_mock_blockchain_and_slips(&keypair, 10).await;
        slips.iter().for_each(|(slip_id, _output_slip)| {
            let receiver = blockchain.utxoset.get_receiver_for_inputs(&vec![*slip_id]);
            assert_eq!(receiver.unwrap(), keypair.public_key());
        });

        let (slip_id, output_slip) = slips[0];
        let mock_tx = test_utilities::make_mock_tx(
            slip_id,
            output_slip.amount(),
            keypair.public_key().clone(),
        );
        let first_block = blockchain.latest_block().unwrap();
        let new_block = test_utilities::make_mock_block_with_tx(
            first_block.hash(),
            first_block.id() + 1,
            mock_tx,
        );
        let is_slip_spendable = blockchain
            .utxoset
            .is_slip_spendable_at_block(&slip_id, new_block.id());
        assert!(is_slip_spendable);
    }

    #[tokio::test]
    async fn output_slip_from_slip_id_test() {
        let keypair = Keypair::new();
        let (blockchain, slips) =
            test_utilities::make_mock_blockchain_and_slips(&keypair, 10).await;
        slips.iter().for_each(|(slip_id, output_slip)| {
            let receiver = blockchain.utxoset.output_slip_from_slip_id(slip_id);
            assert_eq!(receiver.unwrap(), output_slip);
        });
    }

    #[tokio::test]
    async fn transaction_routing_work_test() {
        let keypair = Keypair::new();
        let (blockchain, mut slips) =
            test_utilities::make_mock_blockchain_and_slips(&keypair, 2).await;

        let (input, _) = slips.pop().unwrap();
        let mut tx = test_utilities::make_mock_tx(input, 100, keypair.public_key().clone());

        let keypair_bytes = keypair.public_key().serialize().to_vec();
        let sig = keypair.sign_message(&hash_bytes(&keypair_bytes));

        tx.add_hop_to_path(Hop::new(keypair.public_key().clone(), sig.clone()));

        let mut fees = blockchain.utxoset.transaction_routing_fees(&tx);

        assert_eq!(2499999999900, fees);

        tx.add_hop_to_path(Hop::new(keypair.public_key().clone(), sig.clone()));

        fees = blockchain.utxoset.transaction_routing_fees(&tx);

        assert_eq!(624999999975, fees);
    }

    #[tokio::test]
    async fn transaction_fees_test() {
        let keypair = Keypair::new();
        let (blockchain, mut slips) =
            test_utilities::make_mock_blockchain_and_slips(&keypair, 2).await;

        let (input, _) = slips.pop().unwrap();
        let tx = test_utilities::make_mock_tx(input, 100, keypair.public_key().clone());

        let fees = blockchain.utxoset.transaction_fees(&tx.core);
        assert_eq!(2499999999900, fees);
    }
}
