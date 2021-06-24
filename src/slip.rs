use crate::crypto::{SaitoPublicKey,SaitoSignature};
use serde::{Deserialize, Serialize};

//
// SlipType is a human-readable indicator of the slip-type, such 
// as in a normal transaction, a VIP-transaction, a rebroadcast
// transaction or a golden ticket, etc.
//
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum SlipType {
    Normal,
}


//
// SlipCore is a self-contained object containing only the minimal 
// information needed about a slip. It exists to simplify block
// serialization and deserialization until we have custom functions
// that handle this at speed.
//
// This is a private variable. Access to variables within the SlipCore
// should be handled through getters and setters in the block which
// surrounds it.
//
#[serde_with::serde_as]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct SlipCore {
    #[serde_as(as = 	"[_; 33]")]
    publickey: 		SaitoPublicKey,
    #[serde_as(as = 	"[_; 64]")]
    uuid: 		SaitoSignature,
    amount: 		u64,
    slip_ordinal:	u8,
    slip_type: 		SlipType,
}
impl SlipCore {
    pub fn new() -> SlipCore {
        SlipCore {
            publickey:  [0;33],
            uuid:      	[0;64],
            amount:    	0,
	    slip_ordinal: 0,
            slip_type:  	SlipType::Normal,
        }
    }
}


#[serde_with::serde_as]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Slip {
    core: SlipCore,
    #[serde_as(as = "[_; 47]")]
    utxoset_key: [u8;47],
}

impl Slip {

    pub fn new() -> Self {
        Slip {
            core: SlipCore::new(),
            utxoset_key: [0;47],
        }
    }

    pub fn get_publickey(&self) -> SaitoPublicKey {
        self.core.publickey
    }

    pub fn get_amount(&self) -> u64 {
        self.core.amount
    }

    pub fn get_uuid(&self) -> SaitoSignature {
        self.core.uuid
    }

    pub fn get_slip_ordinal(&self) -> u8 {
        self.core.slip_ordinal
    }

    pub fn get_slip_type(&self) -> SlipType {
        self.core.slip_type
    }

    pub fn set_publickey(&mut self, publickey : SaitoPublicKey) {
        self.core.publickey = publickey;
    }

    pub fn set_amount(&mut self, amount : u64) {
        self.core.amount = amount;
    }

    pub fn set_uuid(&mut self, uuid : SaitoSignature) {
        self.core.uuid = uuid;
    }

    pub fn set_slip_ordinal(&mut self, slip_ordinal : u8) {
        self.core.slip_ordinal = slip_ordinal;
    }

    pub fn set_slip_type(&mut self, slip_type : SlipType) {
        self.core.slip_type = slip_type;
    }


    //
    // when users sign transactions, they must be signing 
    //
    pub fn serialize_for_transaction_signature(&self) -> Vec<u8> {

	let mut vbytes : Vec<u8> = vec![];
	        vbytes.extend(&self.core.publickey);
	        vbytes.extend(&self.core.uuid);
	        vbytes.extend(&self.core.amount.to_be_bytes());
	        vbytes.extend(&(self.core.slip_type as u32).to_be_bytes());

	return vbytes;

    }

}




