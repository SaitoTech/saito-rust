use crate::{block::Block, transaction::Transaction};

#[derive(Clone, Debug)]
pub enum SaitoMessage {
    Transaction { payload: [u8;32] },
    Block { payload: [u8;32] },
    StartBundling,
    StopBundling,
}
