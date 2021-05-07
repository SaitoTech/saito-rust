use crate::block::{Block, BlockHeader};
use crate::burnfee::BurnFee;
use crate::config::GENESIS_PERIOD;
use crate::blockchain::Blockchain;
use crate::golden_ticket::GoldenTicket;
use crate::transaction::{Transaction, TransactionBroadcastType};
use crate::types::SaitoMessage;
use crate::wallet::Wallet;
use crate::helper::{create_timestamp, format_timestamp};

use tokio::sync::{broadcast};
use std::sync::{Arc, RwLock, Mutex};
#[derive(Debug)]
pub struct Mempool {
    wallet: Wallet,
    blocks: Vec<Block>,
    pub transactions: Vec<Transaction>,
    burnfee: BurnFee,
    work_available: u64,
    saito_message_rx: broadcast::Receiver<SaitoMessage>,
    saito_message_tx: broadcast::Sender<SaitoMessage>,
}

impl Mempool {
    pub fn new(
        wallet: Wallet,
        rx: broadcast::Receiver<SaitoMessage>,
        tx: broadcast::Sender<SaitoMessage>,
    ) -> Self {
        return Mempool {
            wallet,
            blocks: vec![],
            transactions: vec![],
            burnfee: BurnFee::new(0.0, 0),
            work_available: 0,
            saito_message_rx: rx,
            saito_message_tx: tx,
        };
    }

    pub async fn run(
        &mut self,
        blockchain: &Arc<Mutex<Blockchain>>,
    ) -> crate::Result<()> {
        loop {
            while let Ok(message) = self.saito_message_rx.recv().await {
                println!("{:?}", message.clone());
                self.process(message, blockchain);
            }
        }
    }

    pub fn process(&mut self, message: SaitoMessage, blockchain: &Arc<Mutex<Blockchain>>) {
        match message {
            SaitoMessage::Transaction { payload } => {
                println!("ADDING TX TO MEMPOOL");
                self.transactions.push(payload);
                self.try_bundle(blockchain);
            },
            SaitoMessage::TryBundle { payload } => {
                self.try_bundle(blockchain);
            },
            _ => {},
        }
    }

    pub fn try_bundle(
        &mut self,
        blockchain: &Arc<Mutex<Blockchain>>,
    ) {
        let latest_block_header = blockchain.lock().unwrap().get_latest_block_header();
        if self.can_bundle_block(latest_block_header.clone()) {
            let block = self.bundle_block( latest_block_header);
            println!("CREATING BLOCK!");
            self.saito_message_tx.send(SaitoMessage::CandidateBlock { payload : block})
                .expect("Err: Could not send block to lottery game");
        }
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

    pub fn can_bundle_block (&mut self, block_header: Option<BlockHeader>) -> bool {
        match block_header {
            Some(block_header) => {
                let timestamp = create_timestamp();
                let work_needed = BurnFee::return_work_needed(
                    block_header.ts,
                    timestamp,
                    10.0,
                );
                println!(
                    "TS: {} -- WORK ---- {:?} -- {:?} --- TX COUNT {:?}",
                    format_timestamp(timestamp),
                    work_needed,
                    self.work_available,
                    self.transactions.len(),
                );

                self.work_available >= work_needed &&
                self.transactions.len() > 0
            },
            None => true
        }
    }

    pub fn bundle_block (
        &mut self,
        previous_block_header: Option<BlockHeader>
    ) -> Block {
        let mut block: Block;
        let publickey = self.wallet.get_publickey();
        let new_burnfee: BurnFee;

        // set the majority of values if we have a previous block header
        match previous_block_header.clone() {
            Some(previous_block_header) => {
                block = Block::new(publickey,previous_block_header.bsh);

                let treasury = previous_block_header.treasury + previous_block_header.reclaimed;
                let coinbase = (treasury as f64 / GENESIS_PERIOD as f64).round() as u64;

                block.set_id(previous_block_header.bid + 1);
                block.set_mintid(previous_block_header.mintid);
                block.set_maxtid(previous_block_header.maxtid);
                block.set_coinbase(coinbase);
                block.set_treasury(treasury - coinbase);
                block.set_prevhash(previous_block_header.bsh);
                block.set_difficulty(previous_block_header.difficulty);
                block.set_paysplit(previous_block_header.paysplit);

                new_burnfee = BurnFee::adjust_work_needed(
                    previous_block_header,
                    block.body.ts,
                );
            },
            None => {
                block = Block::new(publickey, [0; 32]);
                new_burnfee = BurnFee::new(10.0, 0);
            }
        }

        // swap transactions with mempool
        block.set_transactions(&mut self.transactions);

        // set burnfee
        block.set_burnfee(new_burnfee);

        // calculate difficulty and paysplit
        match previous_block_header {
            Some(previous_block_header) => {
                // if there's no goldenticket, default to past values
                let mut new_difficulty = previous_block_header.difficulty;
                let mut new_paysplit = previous_block_header.paysplit;

                for tx in block.body.txs.iter() {
                    if tx.get_tx_type() == TransactionBroadcastType::GoldenTicket {
                        let msg = &tx.body.msg;
                        let gt: GoldenTicket  = bincode::deserialize(&msg[..]).unwrap();
                        new_difficulty = gt.calculate_difficulty(previous_block_header.difficulty);
                        new_paysplit = gt.calculate_paysplit(previous_block_header.paysplit);
                    }
                }

                block.set_difficulty(new_difficulty);
                block.set_paysplit(new_paysplit);

            },
            None => {},
        }

        return block;

    }
}