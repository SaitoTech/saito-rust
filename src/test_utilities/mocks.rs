use crate::{
    block::Block,
    blockchain::Blockchain,
    burnfee::BurnFee,
    crypto::{generate_random_bytes, hash, SaitoHash},
    golden_ticket::GoldenTicket,
    miner::Miner,
    slip::{Slip, SlipType},
    transaction::Transaction,
    time::{create_timestamp},
    wallet::Wallet,
};

use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn make_mock_blockchain(
    wallet_lock: Arc<RwLock<Wallet>>,
    chain_length: u64,
) -> (Arc<RwLock<Blockchain>>, Vec<SaitoHash>) {
    let mut block_hashes: Vec<SaitoHash> = vec![];
    let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
    let mut miner = Miner::new(wallet_lock.clone());

    let current_block_hash = [0; 32];
    let mut transactions: Vec<Transaction>;
    let mut last_block_hash: SaitoHash = [0; 32];
    let mut last_block_difficulty: u64 = 0;
    let publickey;

    let mut test_block_hash: SaitoHash;
    let mut test_block_id: u64;

    {
        let wallet = wallet_lock.read().await;
        publickey = wallet.get_publickey();
    }

    for i in 0..chain_length as u64 {
        transactions = vec![];
        let block: Block;

        // first block
        if i == 0 {
            let mut tx = Transaction::generate_vip_transaction(
                wallet_lock.clone(),
                publickey,
                10_000_000,
            )
            .await;
            tx.generate_metadata(publickey);
            transactions.push(tx);

            block = Block::generate(
                &mut transactions,
                current_block_hash,
                wallet_lock.clone(),
                blockchain_lock.clone(),
            )
            .await;

            last_block_hash = block.get_hash();
            last_block_difficulty = block.get_difficulty();

            block_hashes.push(last_block_hash);

        // second block
        } else {
            // generate golden ticket
            let golden_ticket: GoldenTicket = miner
                .mine_on_block_until_golden_ticket_found(last_block_hash, last_block_difficulty)
                .await;

            let mut transaction: Transaction;

            {
                let mut wallet = wallet_lock.write().await;
                transaction = wallet.create_golden_ticket_transaction(golden_ticket).await;
            }

            transaction.generate_metadata(publickey);
            transactions.push(transaction);

            {
                let blockchain = blockchain_lock.read().await;
                last_block_hash = blockchain.get_latest_block().unwrap().get_hash();
                last_block_difficulty = blockchain.get_latest_block().unwrap().get_difficulty();

                block_hashes.push(last_block_hash);
            }

            let future_timestamp = create_timestamp() + (i * 120000);

            block = Block::generate_with_timestamp(
                &mut transactions,
                last_block_hash,
                wallet_lock.clone(),
                blockchain_lock.clone(),
                future_timestamp,
            )
            .await;
        }

        test_block_hash = block.get_hash();
        test_block_id = block.get_id();

        {
            let mut blockchain = blockchain_lock.write().await;
            blockchain.add_block(block).await;
        }
    }

    (blockchain_lock, block_hashes)
}

pub fn make_mock_block(
    prev_timestamp: u64,
    previous_burnfee: u64,
    previous_block_hash: SaitoHash,
    block_id: u64,
) -> Block {
    let step: u64 = 10000;
    let timestamp = prev_timestamp + step;
    let burnfee = BurnFee::return_burnfee_for_block_produced_at_current_timestamp_in_nolan(
        previous_burnfee,
        timestamp,
        prev_timestamp,
    );
    let wallet = Wallet::new();
    let mut mock_input = Slip::new();
    mock_input.set_publickey(wallet.get_publickey());
    mock_input.set_uuid([0; 32]);
    mock_input.set_amount(1);
    mock_input.set_slip_ordinal(0);
    mock_input.set_slip_type(SlipType::Normal);

    let mut mock_output = Slip::new();
    mock_output.set_publickey(wallet.get_publickey());
    mock_output.set_uuid([0; 32]);
    mock_output.set_amount(1);
    mock_output.set_slip_ordinal(0);
    mock_output.set_slip_type(SlipType::Normal);

    let mut transaction = Transaction::new();

    transaction.add_input(mock_input);
    transaction.add_output(mock_output);

    transaction.sign(wallet.get_privatekey());

    let mut block = Block::new();

    block.set_id(block_id);
    block.set_timestamp(timestamp);
    block.set_previous_block_hash(previous_block_hash);
    block.set_creator(wallet.get_publickey());
    block.set_merkle_root([2; 32]);
    block.set_signature([3; 64]);
    block.set_difficulty(0);
    block.set_treasury(0);
    block.set_burnfee(burnfee);

    block.set_transactions(&mut vec![transaction]);
    block.set_merkle_root(block.generate_merkle_root());
    block.set_hash(block.generate_hash());

    block
}

pub async fn make_mock_tx(wallet_mutex: Arc<RwLock<Wallet>>) -> Transaction {
    let wallet = wallet_mutex.read().await;
    let mut transaction = Transaction::new();
    transaction.set_message((0..1024).map(|_| rand::random::<u8>()).collect());
    let wallet_publickey = wallet.get_publickey();
    let wallet_privatekey = wallet.get_privatekey();
    let mut input1 = Slip::new();
    input1.set_publickey(wallet_publickey);
    input1.set_amount(1000000);
    let random_uuid = hash(&generate_random_bytes(32));
    input1.set_uuid(random_uuid);

    let mut output1 = Slip::new();
    output1.set_publickey(wallet_publickey);
    output1.set_amount(1000000);
    output1.set_uuid([0; 32]);

    transaction.add_input(input1);
    transaction.add_output(output1);

    transaction.sign(wallet_privatekey);
    transaction
}
