use crate::{
    block::Block,
    transaction::Transaction,
    golden_ticket::GoldenTicket,
};

#[derive(Clone, Debug)]
pub enum SaitoMessage {
    Block {
        payload: Block
    },
    Transaction {
        payload: Transaction
    },
    GoldenTicket {
        payload: GoldenTicket
    }
}