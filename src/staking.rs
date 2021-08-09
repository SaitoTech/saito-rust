use crate::{
    block::Block,
    blockchain::{Blockchain, GENESIS_PERIOD},
    crypto::{SaitoHash},
    slip::Slip,
    wallet::Wallet,
};
use bigint::uint::U256;

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
    pub fn reset_staker_table(&mut self , staking_treasury: u64) {

	//
        // move pending into staking table
	//
	for i in 0..self.pending.len() { self.add_staker(self.pending[i].clone()); }
	for i in 0..self.deposits.len() { self.add_staker(self.deposits[i].clone()); }
	self.pending = vec![];
	self.deposits = vec![];

	if self.stakers.len() == 0 {
	    return;
	}

	//
	// adjust the slip amounts based on genesis period
	//
	let staking_payout_per_block : u64 = staking_treasury / GENESIS_PERIOD;

println!("expected payout per block:  {}", staking_payout_per_block);

	//
	// calculate average amount staked
	//
	let one_nolan: u64 = 100_000_000;
	let mut total_staked: u64 = 0;
	for i in 0..self.stakers.len() { total_staked += self.stakers[i].get_amount(); }
	let average_staked = total_staked / self.stakers.len() as u64;

	//
	// calculate the payout for average stake
	//
println!("average staked: {}", average_staked);
	let m = U256::from_big_endian(&staking_payout_per_block.to_be_bytes());
	let p = U256::from_big_endian(&self.stakers.len().to_be_bytes());

	let (q, r) = m.overflowing_div(p);
	let average_staker_payout = q.as_u64();

println!("average payout: {}", average_staker_payout);

	//
	// and adjust the payout based on this....
	//
	for i in 0..self.stakers.len() { 

println!("processing staker: {}", i);

	    //
	    // get the total staked
	    //
	    let my_staked_amount = self.stakers[i].get_amount();

println!("staker has: {}", my_staked_amount);

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

	    let my_payout = my_staked_amount + staking_profit;

	    self.stakers[i].set_amount(my_payout);

println!("The payout for staker {} with {} is {}", i, my_staked_amount, my_payout);

	}
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



    pub fn on_chain_reorganization(
        &mut self,
	block: &Block,
        longest_chain: bool,
    ) -> bool {

	//
	// update staking tables
	//
	if block.get_has_fee_transaction() && block.get_has_golden_ticket() {

	    if longest_chain {

		//
		// roll forward
		//
		let fee_transaction = &block.transactions[block.get_fee_transaction_idx() as usize];

		for i in 0..fee_transaction.outputs.len() {

		    //
		    // staking payout
		    //
		    if i == 3 {

		    }
		}

		// roll forward!
		println!("roll forward...");


	    } else {

		// roll backward!
		println!("roll backward...");

	    }
	}
        true
    }

}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{
        block::Block,
	slip::{Slip, SlipType},
    };

    #[test]
    fn staking_table_test() {

	let mut staking = Staking::new();

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

println!("staking deposits: {}", staking.deposits.len());

	staking.reset_staker_table(1_000_000_000); // 10 Saito

println!("staking stakers: {}", staking.stakers.len());

	assert_eq!(staking.stakers[0].get_amount(), 210000000);
	assert_eq!(staking.stakers[1].get_amount(), 315000000);
	assert_eq!(staking.stakers[2].get_amount(), 420000000);
	assert_eq!(staking.stakers[3].get_amount(), 525000000);
	assert_eq!(staking.stakers[4].get_amount(), 630000000);
    }

}
