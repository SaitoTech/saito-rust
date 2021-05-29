use crate::block::Block;

// TODO put these sort of consts into a single location.
pub const EPOCH_LENGTH: usize = 30_000;

const RING_BUFFER_LENGTH: usize = 2 * EPOCH_LENGTH;

// TODO Put this into crypto and use it everywhere we have [u8; 32]
pub type SECP256K1Hash = [u8; 32];

lazy_static! {
    static ref epoch_ring_array: [SECP256K1Hash; RING_BUFFER_LENGTH] = [0; RING_BUFFER_LENGTH];
    static ref epoch_ring_top_location: u64 = 0; // like a stack pointer
    static ref epoch_ring_length: u64 = 0; // This should either be the lenght of the blockchain or 2x EPOCH_LENGTH(RING_BUFFER_LENGTH)
}

// The epoch should be stored in a ring. The location pointer is the index of the latest block.
// When the length of the blockchain begins to exceed 2x epoch length new blocks will begin to
// overwrite older blocks. During a rollback those blocks which fell off won't be recovered.
// I don't think we need to store these as Option<SECP256K1Hash> because the top_location + length
// can tell us where the valid data is.

pub fn roll_back() -> SECP256K1Hash {
    println!("rollback");
}
pub fn roll_forward(new_block_hash: SECP256K1Hash) {
    println!("rollfoward");
}
pub fn get_block_hash_by_id(id: u64) -> SECP256K1Hash {
    println!("get_block_hash_by_id");
}
pub fn get_latest_block() -> SECP256K1Hash {
    println!("get_latest_block");
}
pub fn get_last_block_in_epoch() -> SECP256K1Hash {
    println!("get_last_block_in_epoch");
}

#[cfg(test)]
mod test {
    use super::*;
    use hex;

    #[test]
    fn test_longest_chain_queue() {
        assert!(false);
    }
}
