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
    publickey: 		[u8;33],
    #[serde_as(as = 	"[_; 64]")]
    uuid: 		[u8;64],
    amount: 		u64,
    sliptype: 		SlipType,
}
impl SlipCore {
    pub fn new() -> SlipCore {
        SlipCore {
            publickey: 	[0;33],
            uuid:      	[0;64],
            amount:    	0,
            sliptype:  	SlipType::Normal,
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


    pub fn serialize_for_signature(&self) -> Vec<u8> {

	let mut vbytes : Vec<u8> = vec![];
	        vbytes.extend(&self.core.publickey);
	        vbytes.extend(&self.core.uuid);
	        vbytes.extend(&self.core.amount.to_be_bytes());
	        vbytes.extend(&(self.core.sliptype as u32).to_be_bytes());

	return vbytes;

    }

}



