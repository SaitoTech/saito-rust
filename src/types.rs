use crate::{
    block::Block,
    transaction::Transaction
};

#[derive(Clone, Debug)]
pub enum SaitoMessage {
    CandidateBlock {
        payload: Block
    },
    NewTargetBlock {
        payload: Block
    },
    Transaction {
        payload: Transaction
    },
    TryBundle {
        payload: bool
    }
}