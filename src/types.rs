use crate::{block::Block, transaction::{TransactionBody, SignedTransaction}};

#[derive(Clone, Debug)]
pub enum SaitoMessage {
    SignedTransaction { payload: SignedTransaction },
    NewBlock { payload: Block },
    TryBundle,
}
