use crate::{block::Block, transaction::Transaction};

#[derive(Clone, Debug)]
pub enum SaitoMessage {
    Transaction { payload: Transaction },
    NewBlock { payload: Block },
    TryBundle,
}
