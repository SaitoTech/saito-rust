// pub mod slip;
// pub mod transaction;
// pub mod blockchain;
// pub mod utxoset;
// pub mod hashminer;
// pub mod saito_router ;
// pub mod mempool;
// pub mod peer_pool;
// pub mod peer;
// pub mod keypair_store;
// pub mod keypair;
// pub mod crypto;
// pub mod peer_policy;
// pub mod concrete_peer_policy;
// pub mod consensus_policy;
// pub mod concrete_consensus_policy;
// pub mod fork_policy;
// pub mod concrete_fork_policy;

pub mod block;
pub mod blockchain;
pub mod burnfee;
pub mod config;
pub mod crypto;
pub mod golden_ticket;
pub mod helper;
pub mod hop;
pub mod mempool;
pub mod lottery;
pub mod slip;
pub mod storage;
pub mod transaction;
pub mod types;
pub mod wallet;
pub mod utxoset;

/// Error returned by most functions.
///
/// When writing a real application, one might want to consider a specialized
/// error handling crate or defining an error type as an `enum` of causes.
/// However, for our example, using a boxed `std::error::Error` is sufficient.
///
/// For performance reasons, boxing is avoided in any hot path. For example, in
/// `parse`, a custom error `enum` is defined. This is because the error is hit
/// and handled during normal execution when a partial frame is received on a
/// socket. `std::error::Error` is implemented for `parse::Error` which allows
/// it to be converted to `Box<dyn std::error::Error>`.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// A specialized `Result` type for mini-redis operations.
///
/// This is defined as a convenience.
pub type Result<T> = std::result::Result<T, Error>;