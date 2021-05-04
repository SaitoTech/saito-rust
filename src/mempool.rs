use crate::block::Block;
use crate::burnfee::BurnFee;
use crate::transaction::Transaction;
use crate::helper::create_timestamp;

#[derive(Debug, Clone)]
pub struct Mempool {
    blocks: Vec<Block>,
    pub transactions: Vec<Transaction>,
    burnfee: BurnFee,
    work_available: u64,
}

impl Mempool {
    pub fn new() -> Mempool {
        return Mempool {
            blocks: vec![],
            transactions: vec![],
            burnfee: BurnFee::new(0.0, 0),
            work_available: 0,
        };
    }

    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
    }

    pub fn add_transaction(&mut self, tx: Transaction) {
        // self.work_available = tx.return_work_available("11413212312313321");
        self.transactions.push(tx.clone());
    }

    pub fn clear_transactions(&mut self) {
        self.transactions = vec![];
        self.work_available = 0;
    }

    // pub fn bundle_block(&mut self) -> Block {
    //     Block {}
    // }
}