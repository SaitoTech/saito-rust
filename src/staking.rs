use crate::{
    block::Block,
    slip::Slip,
    wallet::Wallet,
};


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

    pub async fn add_deposit(&mut self, slip : Slip) {
	self.deposits.push(slip);
    }

    pub async fn add_staker(&mut self, slip : Slip) {
	self.stakers.push(slip);
    }

    pub async fn add_pending(&mut self, slip : Slip) {
	self.pending.push(slip);
    }


    pub async fn remove_deposit(&mut self, slip : Slip) -> bool {
	for i in 0..self.deposits.len() {
	    if slip.get_utxoset_key() == self.deposits[i].get_utxoset_key() {
		let _removed_slip = self.deposits.remove(i);    
		return true;
	    }
        }
	return false;
    }


    pub async fn remove_staker(&mut self, slip : Slip) -> bool {
	for i in 0..self.stakers.len() {
	    if slip.get_utxoset_key() == self.stakers[i].get_utxoset_key() {
		let _removed_slip = self.stakers.remove(i);    
		return true;
	    }
        }
	return false;
    }

    pub async fn remove_pending(&mut self, slip : Slip) -> bool {
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

mod test {}
