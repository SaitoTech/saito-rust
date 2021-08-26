use crate::{
    block::Block,
    blockchain::{GENESIS_PERIOD},
    crypto::{hash, SaitoHash, SaitoUTXOSetKey},
    golden_ticket::GoldenTicket,
    slip::{Slip, SlipType},
    transaction::{Transaction, TransactionType},
};
use bigint::uint::U256;
use ahash::AHashMap;

#[derive(Debug, Clone)]
pub struct Staking {
    // deposits waiting to join staking table for the first time
    pub deposits: Vec<Slip>,
    // in staking table waiting for selection / payout
    pub stakers: Vec<Slip>,
    // waiting for reset of staking table
    pub pending: Vec<Slip>,
}

impl Staking {
    pub fn new() -> Staking {
        Staking {
	    deposits: vec![],
	    stakers: vec![],
	    pending: vec![],
        }
    }

    pub fn add_staker_with_number(&mut self, slip : Slip, random_number : SaitoHash) {

        //
        // find winning nolan
        //
        let x = U256::from_big_endian(&random_number);
	let y = self.stakers.len() + 1;
        let z = U256::from_big_endian(&y.to_be_bytes());
        let (zy, _bolres) = x.overflowing_rem(z);

        let insert_into_pos = zy.low_u64();

	self.stakers.insert(insert_into_pos as usize, slip);

    }

    pub fn find_winning_staker(&self, random_number : SaitoHash) -> Option<Slip> {

        if self.stakers.len() == 0 { return None; }

        //
        // find winning staker
        //
        let x = U256::from_big_endian(&random_number);
	let y = self.stakers.len();
        let z = U256::from_big_endian(&y.to_be_bytes());
        let (zy, _bolres) = x.overflowing_rem(z);

        let retrieve_from_pos = zy.low_u64();

	let winning_slip = self.stakers[retrieve_from_pos as usize].clone();

	return Some(winning_slip);    
    }


    //
    // resets the staker table
    //
    // without using floating-point division, we calculate the share that each staker
    // should earn of the upcoming sweep through the stakers table, and insert the 
    // pending and pending-deposits slips into the staking table with the updated
    // expected payout.
    //
    // returns three vectors with slips to SPEND, UNSPEND, DELETE
    //
    pub fn reset_staker_table(&mut self , staking_treasury: u64) -> (Vec<Slip>, Vec<Slip>, Vec<Slip>) {

println!("===========================");
println!("=== RESET STAKING TABLE ===");
println!("===========================");

	let res_spend: Vec<Slip> = vec![];
	let res_unspend: Vec<Slip> = vec![];
	let res_delete: Vec<Slip> = vec![];

	//
        // move pending into staking table
	//
	for i in 0..self.pending.len() { self.add_staker(self.pending[i].clone()); }
	for i in 0..self.deposits.len() { self.add_staker(self.deposits[i].clone()); }
	self.pending = vec![];
	self.deposits = vec![];

	if self.stakers.len() == 0 {
	    return (res_spend, res_unspend, res_delete);
	}

	//
	// adjust the slip amounts based on genesis period
	//
	let staking_payout_per_block : u64 = staking_treasury / GENESIS_PERIOD;

	//
	// calculate average amount staked
	//
	let mut total_staked: u64 = 0;
	for i in 0..self.stakers.len() {
	    // TODO - delete when ready
	    // we do not update the slip type anymore because we need
	    // the slip to be spendable when it is issued.
	    // we update the payout rather than the amount so the slip
	    // can validate as spendable as well.
	    self.stakers[i].set_slip_type(SlipType::StakerOutput);
	    total_staked += self.stakers[i].get_amount();
	}
	let average_staked = total_staked / self.stakers.len() as u64;

	//
	// calculate the payout for average stake
	//
	let m = U256::from_big_endian(&staking_payout_per_block.to_be_bytes());
	let p = U256::from_big_endian(&self.stakers.len().to_be_bytes());

	let (q, _r) = m.overflowing_div(p);
	let average_staker_payout = q.as_u64();

	//
	// and adjust the payout based on this....
	//
	for i in 0..self.stakers.len() { 

	    //
	    // get the total staked
	    //
	    let my_staked_amount = self.stakers[i].get_amount();

	    //
	    // figure how much we are due...
	    //
	    // my stake PLUS (my stake / 1 * ( my_stake / average_staked )
	    // my stake PLUS (my stake / 1 * ( my_stake / average_staked ) * ( ( treasury / genesis_period )
	    // my stake PLUS (my stake / 1 * ( my_stake / average_staked ) * ( ( treasury / genesis_period )
	    //
	    let a = U256::from_big_endian(&my_staked_amount.to_be_bytes());
	    let b = U256::from_big_endian(&average_staker_payout.to_be_bytes());
	    let nominator : U256 = a.saturating_mul(b);
	    let denominator = U256::from_big_endian(&average_staked.to_be_bytes());

	    let (z, f)  = nominator.overflowing_div(denominator);

	    let mut staking_profit: u64 = 0;
	    if f != true { staking_profit = z.as_u64(); }

	    self.stakers[i].set_payout(staking_profit);

	}

        return (res_spend, res_unspend, res_delete);
    }



    pub fn add_deposit(&mut self, slip : Slip) {
	self.deposits.push(slip);
    }

    pub fn add_staker(&mut self, slip : Slip) {
	self.stakers.push(slip);
    }

    pub fn add_pending(&mut self, slip : Slip) {
	self.pending.push(slip);
    }


    pub fn remove_deposit(&mut self, slip : Slip) -> bool {
	for i in 0..self.deposits.len() {
	    if slip.get_utxoset_key() == self.deposits[i].get_utxoset_key() {
		let _removed_slip = self.deposits.remove(i);    
		return true;
	    }
        }
	return false;
    }


    pub fn remove_staker(&mut self, slip : Slip) -> bool {
	for i in 0..self.stakers.len() {
	    if slip.get_utxoset_key() == self.stakers[i].get_utxoset_key() {
		let _removed_slip = self.stakers.remove(i);    
		return true;
	    }
        }
	return false;
    }

    pub fn remove_pending(&mut self, slip : Slip) -> bool {
	for i in 0..self.pending.len() {
	    if slip.get_utxoset_key() == self.pending[i].get_utxoset_key() {
		let _removed_slip = self.pending.remove(i);    
		return true;
	    }
        }
	return false;
    }



    //
    // handle staking / pending / deposit tables
    //
    // returns slips to SPEND, UNSPEND and DELETE
    // 
    // this is required as staking table controlled by blockchain and Rust
    // restricts us from passing the UTXOSET into this part of the program.
    //
    pub fn on_chain_reorganization(
        &mut self,
	block: &Block,
        longest_chain: bool,
    ) -> (Vec<Slip>, Vec<Slip>, Vec<Slip>) {

	let res_spend: Vec<Slip> = vec![];
	let res_unspend: Vec<Slip> = vec![];
	let res_delete: Vec<Slip> = vec![];

	//
	// add/remove deposits
	//
        for tx in &block.transactions {
            if tx.get_transaction_type() == TransactionType::StakerDeposit {
		for i in 0..tx.outputs.len() {
		    if tx.outputs[i].get_slip_type() == SlipType::StakerDeposit {

			//
			// roll forward
			//
		        if longest_chain {
			    self.add_deposit(tx.outputs[i].clone());

			//
			// roll backward
			//
			} else {
			    self.remove_deposit(tx.outputs[i].clone());
			}

		    }
		}
	    }
        }


        //
        // reset tables if needed
        //
        if longest_chain {
	    //
	    // reset stakers if necessary
	    //
	    if self.stakers.len() == 0 {
	       //self.reset_staker_table(block.get_staking_treasury());
	       let (_res_spend, _res_unspend, _res_delete) = self.reset_staker_table(100_000_000);
	    }
        } else {
	    //
	    // reset pending if necessary
	    //
	    if self.pending.len() == 0 {
		self.pending = vec![];
		self.deposits = vec![];
   	        for i in 0..self.stakers.len() {
		    if self.stakers[i].get_slip_type() == SlipType::StakerOutput {
	                self.pending.push(self.stakers[i].clone());
		    }
		    if self.stakers[i].get_slip_type() == SlipType::StakerDeposit {
	                self.deposits.push(self.stakers[i].clone());
		    }
	        }
		self.stakers = vec![];
	    }
	}




	//
	// update staking tables
	//
	if block.get_has_fee_transaction() && block.get_has_golden_ticket() {

	    let fee_transaction = &block.transactions[block.get_fee_transaction_idx() as usize];

	    let golden_ticket_transaction = &block.transactions[block.get_golden_ticket_idx() as usize];

	    //
            // grab random input from golden ticket
            //
            let golden_ticket: GoldenTicket = GoldenTicket::deserialize_for_transaction(
                golden_ticket_transaction.get_message().to_vec(),
            );
            let router_random_number1 = hash(&golden_ticket.get_random().to_vec()); // router block1
	    let staker_random_number = hash(&router_random_number1.to_vec());	// staker block2
	    let _router_random_number2 = hash(&staker_random_number.to_vec());	// router block2

	    if fee_transaction.outputs.len() < 3 { return (res_spend, res_unspend, res_delete); }
	    if fee_transaction.inputs.len() < 1 { return (res_spend, res_unspend, res_delete); }

	    let staker_output = fee_transaction.outputs[2].clone(); // 3rd output is staker
	    let staker_input = fee_transaction.inputs[0].clone(); // 1st input is staker
println!("Block {} has payout: ", &block.get_id());
//println!("staker input:  {:?}", staker_input);
//println!("staker output: {:?}", staker_output);



	    //
	    // roll forward
	    //
	    if longest_chain {

//println!("ok, ready to roll...");

		//
		// re-create staker table, if needed
		//
		// we do this at both the start and the end of this function so that 
		// we will always have a table that can be handled regardless of 
		// vacillations in on_chain_reorg, such as resetting the table and
		// then non-longest-chaining the same block
		//
println!("Rolling forward and moving into pending: {}!", self.stakers.len());
		if self.stakers.len() == 0 {
		    //self.reset_staker_table(block.get_staking_treasury());
		    let (_res_spend, _res_unspend, _res_delete) = self.reset_staker_table(100_000_000);
		}

		//
		// move staker to pending
		//
//println!("staker table is: {:?}", self.stakers);


		let lucky_staker_option = self.find_winning_staker(staker_random_number);
		if let Some(lucky_staker) = lucky_staker_option {
println!("the lucky staker is: {:?}", lucky_staker);
println!("moving from staker into pending: {}", lucky_staker.get_amount());
		    self.remove_staker(lucky_staker.clone());
		    self.add_pending(staker_output.clone());
		}

		//
		// re-create staker table, if needed
		//
		if self.stakers.len() == 0 {
		    //self.reset_staker_table(block.get_staking_treasury());
		    let (_res_spend, _res_unspend, _res_delete) = self.reset_staker_table(100_000_000);
		}


	    //
	    // roll backward
	    //
	    } else {

		println!("roll backward...");

		//
		// reset pending if necessary
		//
	        if self.pending.len() == 0 {
		    self.pending = vec![];
		    self.deposits = vec![];
   	            for i in 0..self.stakers.len() {
		        if self.stakers[i].get_slip_type() == SlipType::StakerOutput {
	                    self.pending.push(self.stakers[i].clone());
		        }
		        if self.stakers[i].get_slip_type() == SlipType::StakerDeposit {
	                    self.deposits.push(self.stakers[i].clone());
		        }
	            }
		    self.stakers = vec![];
	        }


		//
		// remove from pending to staker (awaiting payout)
		//
	  	self.remove_pending(staker_output.clone());
		let slip_type = staker_input.get_slip_type();
		if slip_type == SlipType::StakerDeposit {
		    self.add_deposit(staker_input.clone());
		}
		if slip_type == SlipType::StakerOutput {
		    self.add_staker(staker_input.clone());
		}



		//
		// reset pending if necessary
		//
	        if self.pending.len() == 0 {
	    	    self.pending = vec![];
		    self.deposits = vec![];
   	            for i in 0..self.stakers.len() {
		        if self.stakers[i].get_slip_type() == SlipType::StakerOutput {
	                    self.pending.push(self.stakers[i].clone());
		        }
		        if self.stakers[i].get_slip_type() == SlipType::StakerDeposit {
	                    self.deposits.push(self.stakers[i].clone());
		        }
	            }
		    self.stakers = vec![];
	        }

	    }
	}

        return (res_spend, res_unspend, res_delete);

    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::test_utilities::mocks::make_mock_block_with_info;
    use crate::{
	blockchain::Blockchain,
	slip::{Slip, SlipType},
	time::{create_timestamp},
	wallet::Wallet,
    };
    use tokio::sync::{RwLock};
    use std::{sync::Arc};

    #[test]
    fn staking_table_test() {

        let wallet_lock = Arc::new(RwLock::new(Wallet::default()));
	let mut staking = Staking::new();
	let mut blockchain = Blockchain::new(wallet_lock.clone());

	let mut slip1 = Slip::new();
	slip1.set_amount(200_000_000);
	slip1.set_slip_type(SlipType::StakerDeposit);

	let mut slip2 = Slip::new();
	slip2.set_amount(300_000_000);
	slip2.set_slip_type(SlipType::StakerDeposit);

	let mut slip3 = Slip::new();
	slip3.set_amount(400_000_000);
	slip3.set_slip_type(SlipType::StakerDeposit);

	let mut slip4 = Slip::new();
	slip4.set_amount(500_000_000);
	slip4.set_slip_type(SlipType::StakerDeposit);

	let mut slip5 = Slip::new();
	slip5.set_amount(600_000_000);
	slip5.set_slip_type(SlipType::StakerDeposit);

	staking.add_deposit(slip1);
	staking.add_deposit(slip2);
	staking.add_deposit(slip3);
	staking.add_deposit(slip4);
	staking.add_deposit(slip5);

	let (_res_spend, _res_unspend, _res_delete) = staking.reset_staker_table(1_000_000_000); // 10 Saito

	assert_eq!(staking.stakers[0].get_amount()+staking.stakers[0].get_payout(), 210000000);
	assert_eq!(staking.stakers[1].get_amount()+staking.stakers[1].get_payout(), 315000000);
	assert_eq!(staking.stakers[2].get_amount()+staking.stakers[2].get_payout(), 420000000);
	assert_eq!(staking.stakers[3].get_amount()+staking.stakers[3].get_payout(), 525000000);
	assert_eq!(staking.stakers[4].get_amount()+staking.stakers[4].get_payout(), 630000000);
    }

    #[tokio::test]
    async fn blockchain_roll_forward_staking_table_test() {

        let wallet_lock = Arc::new(RwLock::new(Wallet::default()));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let publickey;
        let mut latest_block_hash = [0; 32];

        {
            let wallet = wallet_lock.read().await;
            publickey = wallet.get_publickey();
        }

	//
	// initialize blockchain staking table
	//
	{
	    let mut blockchain = blockchain_lock.write().await;

   	    let mut slip1 = Slip::new();
	    slip1.set_amount(200_000_000);
	    slip1.set_slip_type(SlipType::StakerDeposit);

	    let mut slip2 = Slip::new();
	    slip2.set_amount(300_000_000);
	    slip2.set_slip_type(SlipType::StakerDeposit);

	    slip1.set_publickey(publickey);
	    slip2.set_publickey(publickey);

	    slip1.generate_utxoset_key();
	    slip2.generate_utxoset_key();

	    slip1.on_chain_reorganization(&mut blockchain.utxoset, true, 0);
	    slip2.on_chain_reorganization(&mut blockchain.utxoset, true, 0);

	    blockchain.staking.add_deposit(slip1);
	    blockchain.staking.add_deposit(slip2);

	    blockchain.staking.reset_staker_table(1_000_000_000); // 10 Saito 	

	}

	//
	// BLOCK 1
	//
	let mut current_timestamp = create_timestamp();
	let mut block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 3, 0, false).await;
        latest_block_hash = block.get_hash();
	Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block, true).await;

	//
	// BLOCK 2
	//
	current_timestamp = create_timestamp() + 120000;
	block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 0, 1, false).await;
        latest_block_hash = block.get_hash();
	Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block, true).await;

	//
	// BLOCK 3
	//
	current_timestamp = create_timestamp() + 240000;
	block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 0, 1, true).await;
        latest_block_hash = block.get_hash();
	Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block, true).await;

	//
	// BLOCK 4
	//
	current_timestamp = create_timestamp() + 360000;
	block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 0, 1, false).await;
        latest_block_hash = block.get_hash();
	Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block, true).await;

	//
	// second staker payment should have happened and staking table reset
	//
        { 
	    let blockchain = blockchain_lock.write().await;
	    let blk = blockchain.get_block(latest_block_hash).await;
	    println!("STAKERS: {:?}", blockchain.staking.stakers);
	    println!("PENDING: {:?}", blockchain.staking.pending);
	    println!("DEPOSIT: {:?}", blockchain.staking.deposits);
	}

	//
	// BLOCK 5
	//
	current_timestamp = create_timestamp() + 480000;
	block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 0, 1, true).await;
        latest_block_hash = block.get_hash();
	Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block, true).await;

	//
	// BLOCK 6
	//
	current_timestamp = create_timestamp() + 600000;
	block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 0, 1, false).await;
        latest_block_hash = block.get_hash();
	Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block, true).await;

	//
	// BLOCK 7
	//
	current_timestamp = create_timestamp() + 720000;
	block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 0, 1, true).await;
        latest_block_hash = block.get_hash();
	Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block, true).await;

	//
	// second staker payment should have happened and staking table reset
	//
        { 
	    let blockchain = blockchain_lock.write().await;
	    let blk = blockchain.get_block(latest_block_hash).await;
	    println!("STAKERS 2: {:?}", blockchain.staking.stakers);
	    println!("PENDING 2: {:?}", blockchain.staking.pending);
	    println!("DEPOSIT 2: {:?}", blockchain.staking.deposits);
	}

	//
	// BLOCK 8
	//
	current_timestamp = create_timestamp() + 840000;
	block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 0, 1, false).await;
        latest_block_hash = block.get_hash();
	Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block, true).await;

	//
	// TEST STAKER PAID
	//
	{
            let blockchain = blockchain_lock.write().await;
	    let blk = blockchain.get_block(latest_block_hash).await;

	    //
	    // second staker payment should have happened and staking table reset
	    //
	    println!("STAKERS 3: {:?}", blockchain.staking.stakers);
	    println!("PENDING 3: {:?}", blockchain.staking.pending);
	    println!("DEPOSIT 3: {:?}", blockchain.staking.deposits);

            //assert_eq!(blk.get_has_fee_transaction(), true);
            //assert_eq!(blk.get_fee_transaction_idx(), 2); // normal tx, golden ticket, fee tx
	    //println!("{:?}", blk.transactions[2].get_outputs());
            //assert_eq!(blk.transactions[2].get_outputs()[2].get_slip_type(), SlipType::StakerOutput);
	}

    }


    #[tokio::test]
    async fn blockchain_staking_deposits_test() {

        let wallet_lock = Arc::new(RwLock::new(Wallet::default()));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let publickey;
        let mut latest_block_hash = [0; 32];

        {
            let wallet = wallet_lock.read().await;
            publickey = wallet.get_publickey();
        }


	//
	// BLOCK 1 -- VIP transactions
	//
	let mut current_timestamp = create_timestamp();
	let mut block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 10, 0, false).await;
        latest_block_hash = block.get_hash();
	Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block, true).await;

	//
	// BLOCK 2 -- staking deposits
	//
        let mut stx1: Transaction;
        let mut stx2: Transaction;
	
        {
            let mut wallet = wallet_lock.write().await;
            stx1 = wallet.create_staking_deposit_transaction(100000).await;
            stx2 = wallet.create_staking_deposit_transaction(200000).await;
	    stx1.generate_metadata(publickey);
	    stx2.generate_metadata(publickey);
        }

	let mut transactions: Vec<Transaction> = vec![];
	transactions.push(stx1);
	transactions.push(stx2);

	current_timestamp = create_timestamp() + 120000;
        block = Block::generate_with_timestamp(
            &mut transactions,
            latest_block_hash,
            wallet_lock.clone(),
            blockchain_lock.clone(),
	    current_timestamp
        ).await;
        latest_block_hash = block.get_hash();
	Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block, true).await;


	//
	// the staking deposits should be econd staker payment should have happened and staking table reset
	//
        { 
	    let blockchain = blockchain_lock.write().await;
	    let blk = blockchain.get_block(latest_block_hash).await;
	    println!("2 staking deposit transactions made, deposits should have TWO");
	    println!("STAKERS: {:?}", blockchain.staking.stakers);
	    println!("PENDING: {:?}", blockchain.staking.pending);
	    println!("DEPOSIT: {:?}", blockchain.staking.deposits);
	}

	//
	// BLOCK 3
	//
	current_timestamp = create_timestamp() + 240000;
	block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 0, 1, true).await;
        latest_block_hash = block.get_hash();
	Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block, true).await;


	//
	// the staking table should have been created when needed for the payout
	//
        { 
	    let blockchain = blockchain_lock.write().await;
	    let blk = blockchain.get_block(latest_block_hash).await;
	    println!("2 staking deposit transactions made... where are we?");
	    println!("STAKERS: {:?}", blockchain.staking.stakers);
	    println!("PENDING: {:?}", blockchain.staking.pending);
	    println!("DEPOSIT: {:?}", blockchain.staking.deposits);
	}


	//
	// BLOCK 4
	//
	current_timestamp = create_timestamp() + 360000;
	block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 0, 1, false).await;
        latest_block_hash = block.get_hash();
	Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block, true).await;


	//
	// BLOCK 5
	//
	current_timestamp = create_timestamp() + 480000;
	block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 0, 1, true).await;
        latest_block_hash = block.get_hash();
	Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block, true).await;

	//
	// the staking table should have been created when needed for the payout
	//
        { 
	    let mut blockchain = blockchain_lock.write().await;
	    let blk = blockchain.get_block(latest_block_hash).await;
	    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
	    println!("STAKERS: {:?}", blockchain.staking.stakers);
	    println!("PENDING: {:?}", blockchain.staking.pending);
	    println!("DEPOSIT: {:?}", blockchain.staking.deposits);
	    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
	    println!("!!! AND NOW REWINDING SHOULD RESTORE STAKING !!!");
	    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");

	    let blk2 = blk.clone();
	    blockchain.staking.on_chain_reorganization(&blk2, false);
	    println!("STAKERS: {:?}", blockchain.staking.stakers);
	    println!("PENDING: {:?}", blockchain.staking.pending);
	    println!("DEPOSIT: {:?}", blockchain.staking.deposits);

	    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
	    println!("!!! AND NOW MOVING FORWARD AGAIN !!!");
	    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");

	    let blk3 = blk2.clone();
	    blockchain.staking.on_chain_reorganization(&blk3, true);
	    println!("STAKERS: {:?}", blockchain.staking.stakers);
	    println!("PENDING: {:?}", blockchain.staking.pending);
	    println!("DEPOSIT: {:?}", blockchain.staking.deposits);



	}



	assert_eq!(0, 1);

    }




}

