use crate::{transaction::Transaction};
use crate::crypto::Sha256Hash;

#[derive(Clone, Debug)]
pub enum SaitoMessage {
    Transaction { payload: Transaction },
    NewBlock { payload: Sha256Hash },
    TryBundle,
}
