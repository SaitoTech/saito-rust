/*!
# Welcome to Saito

Saito is a **Tier 1 Blockchain Protocol** that incentivizes the provision of **high-throughput** network infrastructure. The network accomplishes with a consensus mechanism that pays the nodes in the peer-to-peer network for the collection and sharing of fees.

Saito-Rust is an implementation of Saito Consensus written in Rust for use by high-throughput routing nodes. It aims to offer the simplest and most scalable implementation of Saito Consensus.

If you need to get in touch with us, please reach out anytime.

# Usage

TODO

# How to contribute

TODO

# Contact

The Saito Team
dev@saito.tech

*/
pub mod block;
pub mod blockchain;
pub mod burnfee;
pub mod consensus;
pub mod crypto;
pub mod forktree;
pub mod golden_ticket;
pub mod keypair;
pub mod longest_chain_queue;
pub mod mempool;
pub mod slip;
pub mod storage;
pub mod time;
pub mod transaction;
pub mod types;
pub mod utxoset;

#[macro_use]
extern crate lazy_static;



/// Error returned by most functions.
///
/// When writing a real application, one might want to consider a specialized
/// error handling crate or defining an error type as an `enum` of causes.
/// However, most time using a boxed `std::error::Error` is sufficient.
///
/// For performance reasons, boxing is avoided in any hot path. For example, in
/// `parse`, a custom error `enum` is defined. This is because the error is hit
/// and handled during normal execution when a partial frame is received on a
/// socket. `std::error::Error` is implemented for `parse::Error` which allows
/// it to be converted to `Box<dyn std::error::Error>`.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// A specialized `Result` type for operations.
///
/// This is defined as a convenience.
pub type Result<T> = std::result::Result<T, Error>;


// #[path = "./test/testutilities.rs"]
// mod testutilities;


#[cfg(feature = "test-utilities")]
pub mod test_utilities {
    use crate::transaction::{Transaction, TransactionType};
    use crate::block::Block;
    use crate::slip::{SlipID, OutputSlip};
    use crate::crypto::{PublicKey, Sha256Hash};
    use crate::time::create_timestamp;
    use std::str::FromStr;
    use secp256k1::Signature;
    pub fn shared_code() {
        println!("I'm inside the library")
    }
    
    pub fn make_mock_block(previous_block_hash: Sha256Hash) -> Block{
        let public_key: PublicKey = PublicKey::from_str(
            "0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072",
        )
        .unwrap();
        let from_slip = SlipID::default();
        let to_slip = OutputSlip::default();
        let mut tx = Transaction::new(
            Signature::from_compact(&[0; 64]).unwrap(),
            vec![],
            create_timestamp(),
            vec![from_slip.clone()],
            vec![to_slip.clone()],
            TransactionType::Normal,
            vec![104, 101, 108, 108, 111],
        );
        let mut tx2 = Transaction::default();

        Block::new_mock(previous_block_hash, vec![tx.clone(), tx2.clone()])
    }
}
