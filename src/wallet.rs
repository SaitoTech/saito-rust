use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::slip::{Slip, SlipSpentStatus};
use crate::transaction::{Transaction, TransactionBroadcastType};
use crate::crypto::{SecretKey, PublicKey, Signature, generate_keys, hash, sign};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Wallet {
    body:        WalletBody,
    slips_hmap:  HashMap<[u8; 32], u8>,
    slips_limit: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct WalletBody {
    balance:     u64,
    privatekey:  SecretKey,
    publickey:   PublicKey,
    slips:       Vec<Slip>,
    default_fee: u64,
    version:     f32,
    pending:     Vec<Transaction>,
}

impl Wallet {
    pub fn new() -> Wallet {
        return Wallet {
            body:                        WalletBody::new(),
            slips_hmap:                  HashMap::new(),
            slips_limit:                 10000,
        };
    }

    pub fn return_publickey(&self) -> PublicKey {
        return self.body.publickey;
    }

    fn return_privatekey(&self) -> SecretKey {
        return self.body.privatekey;
    }

    pub fn create_signature(&self, data: &[u8]) -> Signature {
        let mut hashed_data: [u8; 32] = [0; 32];
        hash(data.to_vec(), &mut hashed_data);
        return sign(&hashed_data, &self.return_privatekey());
    }
}

impl WalletBody {
    pub fn new() -> WalletBody {
        let (_privatekey, _publickey) = generate_keys();
        return WalletBody {
            balance:     0,
            privatekey:  _privatekey,
            publickey:   _publickey,
            slips:       vec![],
            default_fee: 200_000_000,
            version:     2.15,
            pending:     vec![],
        };
    }
}