use crate::{block::Block, transaction::TransactionCore};

#[derive(Clone, Debug)]
pub enum SaitoMessage {
    TransactionCore { payload: TransactionCore },
    Block { payload: Block },
    TryBundle,
}
