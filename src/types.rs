use crate::transaction::Transaction;

#[derive(Clone, Debug)]
pub enum SaitoMessage {
    Transaction { payload: Transaction },
    TryBundle,
}
