use crate::{block::Block, transaction::SignedTransaction};

#[derive(Clone, Debug)]
pub enum SaitoMessage {
    SignedTransaction { payload: SignedTransaction },
    NewBlock { payload: Block },
    TryBundle,
}
