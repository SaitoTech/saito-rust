use crate::{
    block::Block,
    blockchain::Blockchain,
    blockring::EPOCH_LENGTH,
    consensus::SaitoMessage,
    crypto::{generate_random_bytes, hash, SaitoHash, SaitoPublicKey},
    golden_ticket::GoldenTicket,
    slip::Slip,
    transaction::{Transaction, TransactionType},
    wallet::Wallet,
};

use std::{sync::Arc, thread::sleep, time::Duration};
use tokio::sync::{broadcast, mpsc, RwLock, RwLockReadGuard};


pub enum MinerMessage {
    StartMining,
    StopMining,
    MineGoldenTicket,
}

pub struct Miner {
    pub is_active: bool,
    pub target: SaitoHash,
}

impl Miner {

    pub fn new() -> Self {
        Self {
            is_active: false,
            target: [0; 32],
        }
    }

    fn is_valid_solution(&self, solution: SaitoHash, previous_block: &Block) -> bool {
        true
    }

    fn find_winner(&self, solution: &SaitoHash, previous_block: &Block) -> SaitoPublicKey {
        [0; 33]
    }

    fn generate_golden_ticket_transaction(
        &self,
        solution: SaitoHash,
        previous_block: &Block,
        wallet: RwLockReadGuard<Wallet>,
    ) -> Transaction {

	return Transaction::default();

/***
        let publickey = wallet.get_publickey();
        let gt_solution = self.create_gt_solution(solution, previous_block.get_hash(), publickey);

        // hardcoded paysplit
        let paysplit = 0.5;

        // Find winning node
        let winning_tx_address = self.find_winner(&solution, &previous_block);

        // we need to calculate the fees that are gonna go in the slips here
        // TODO add burnfee
        // let paid_burn_fee = previous_block.get_paid_burnfee();

        // This is just inputs - outputs for all transactions in the block
        // TODO add fees
        // let total_fees_for_creator = previous_block.get_available_fees(&previous_block.get_creator());

        // get the fees available from our publickey
        // TODO add fees
        // let total_fees_in_block = previous_block.get_available_fees(&publickey);

        // calculate the amount the creator can take for themselves
        // let mut creator_surplus = 0;
        // if total_fees_for_creator > paid_burn_fee {
        //     creator_surplus = total_fees_for_creator - paid_burn_fee;
        // }

        // find the amount that will be divied out to miners and nodes
        // (total_fees_in_block - creator_surplus) + previous_block.get_coinbase();

        let total_fees_for_miners_and_nodes =
            (previous_block.get_treasury() as f64 / EPOCH_LENGTH as f64).round() as u64;

        // Calculate shares
        let miner_share = (total_fees_for_miners_and_nodes as f32 * paysplit).round() as u64;
        let node_share = total_fees_for_miners_and_nodes - miner_share;

        // create our golden ticket tx (au_tx)
        let mut golden_transaction = Transaction::default();
        golden_transaction.set_transaction_type(TransactionType::GoldenTicket);

        let mut miner_slip = Slip::default();
        miner_slip.set_publickey(publickey);
        miner_slip.set_amount(miner_share);

        let mut node_slip = Slip::default();
        node_slip.set_publickey(winning_tx_address);
        node_slip.set_amount(node_share);

        golden_transaction.add_output(miner_slip);
        golden_transaction.add_output(node_slip);
        golden_transaction.set_message(bincode::serialize(&gt_solution).unwrap());

        // sign TX
        golden_transaction.sign(wallet.get_privatekey());
***/
        //return golden_transaction;
    }

    fn create_gt_solution(
        &self,
        random: [u8; 32],
        target: [u8; 32],
        publickey: SaitoPublicKey,
    ) -> GoldenTicket {
        return GoldenTicket::new(1, target, random, publickey);
    }

    pub fn set_target(&mut self, target: SaitoHash) {
        self.target = target;
    }

    pub fn set_is_active(&mut self, is_active: bool) {
        self.is_active = is_active;
    }

/***
            let miner = miner_run_lock.read().await;
            if miner.is_active {
                let blockchain = blockchain_lock.read().await;
                let block = blockchain.get_block(&miner.target).unwrap();
                let solution = hash(&generate_random_data(32));
                if miner.is_valid_solution(solution, block) {
                    let wallet = wallet_lock.read().await;
                    let golden_tx =
                        miner.generate_golden_ticket_transaction(solution, block, wallet);

                    {
                        let mut miner_write = miner_run_lock.write().await;
                        miner_write.set_is_active(false);
                    }

                    broadcast_channel_sender
                        .send(SaitoMessage::MempoolNewTransaction {
                            transaction: golden_tx,
                        })
                        .unwrap();
                }
            }
****/



}

pub async fn run(
    miner_lock: Arc<RwLock<Miner>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    wallet_lock: Arc<RwLock<Wallet>>,
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    mut broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {

    let (miner_channel_sender, mut miner_channel_receiver) = mpsc::channel(4);


    let mine_ticket_sender = miner_channel_sender.clone();
    tokio::spawn(async move {
        loop {
            mine_ticket_sender
                .send(MinerMessage::MineGoldenTicket)
                .await
                .expect("error: GenerateBlock message failed to send");
            sleep(Duration::from_millis(1000));
        }
    });



    loop {
        tokio::select! {

	    //
	    // Miner Channel Messages
	    //
            Some(message) = miner_channel_receiver.recv() => {
                match message {

		    //
		    // Mine 1 Ticket 
		    //
                    MinerMessage::MineGoldenTicket => {
println!("RECEIVED MINE GOLDEN TICKET MSG");
                    },
		    _ => {}
		}
	    }


	    //
	    // Saito Channel Messages
	    //
            Ok(message) = broadcast_channel_receiver.recv() => {
                match message {

                    SaitoMessage::NewLongestChainBlock { hash : block_hash } => {
                        let mut miner = miner_lock.write().await;
                        miner.set_target(block_hash);
                        miner.set_is_active(true);
                    },
		    _ => {}
                }
            }

	}
    }

    Ok(())
}

mod test {}
