use crate::crypto::Sha256Hash;
use crate::transaction::Transaction;

#[derive(Clone, Debug)]
pub enum SaitoMessage {
    Transaction { payload: Transaction },
    NewBlock { payload: Sha256Hash },
    TryBundle,
}
