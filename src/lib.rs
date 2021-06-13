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
pub mod keypair;
pub mod mempool;
pub mod miner;
pub mod path;
pub mod shashmap;
pub mod slip;
pub mod time;
pub mod transaction;
pub mod types;



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
