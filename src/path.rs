use secp256k1::{PublicKey, Signature};
//use serde::{Deserialize, Serialize};


/// A single record used in the history of transactions being routed around the network
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Hop {
    /// An `secp256k1::PublicKey` of the router
    pub publickey: PublicKey,
    /// An `secp256k1::Signature` proving routing work
    pub signature: Signature,
}

impl Hop {
    /// Creates a new `Hop`
    ///
    /// * `address` - `secp256k1::PublicKey` address of router
    /// * `signature` - `secp256k1::Signature` verifying work done by routers
    pub fn new(publickey: PublicKey, signature: Signature) -> Hop {
        return Hop { publickey, signature };
    }
}


/// A single record used in the history of transactions being routed around the network
#[derive(Debug, PartialEq, Clone)]
pub struct Path {
    /// A vector of routing hops
    pub hops: Vec<Hop>,
}

impl Path {
    pub fn new() -> Path {
        return Path {
	  hops: vec![],
	}
    }

    /// Add a new `Hop` to the list of `Hop`s
    pub fn add_hop(&mut self, hop: Hop) {
        self.hops.push(hop);
    }
}

