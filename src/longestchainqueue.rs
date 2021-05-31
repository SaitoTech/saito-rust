// TODO put these sort of consts into a single location.
pub const EPOCH_LENGTH: usize = 30_000;

const RING_BUFFER_LENGTH: usize = 2 * EPOCH_LENGTH;

// TODO Put this into crypto and use it everywhere we have [u8; 32]
pub type SECP256K1Hash = [u8; 32];

lazy_static! {
    static ref EPOCH_RING_ARRAY: [SECP256K1Hash; RING_BUFFER_LENGTH] = [[0; 32]; RING_BUFFER_LENGTH];
    static ref EPOCH_RING_TOP_LOCATION: u64 = 0; // like a stack pointer
    static ref EPOCH_RING_LENGTH: u64 = 0; // This should either be the lenght of the blockchain or 2x EPOCH_LENGTH(RING_BUFFER_LENGTH)
}

// The epoch should be stored in a ring. The location pointer is the index of the latest block.
// When the length of the blockchain begins to exceed 2x epoch length new blocks will begin to
// overwrite older blocks. During a rollback those blocks which fell off won't be recovered.
// I don't think we need to store these as Option<SECP256K1Hash> because the top_location + length
// can tell us where the valid data is.

pub fn roll_back() -> SECP256K1Hash {
    println!("rollback");
    [0; 32]
}
pub fn roll_forward(_new_block_hash: SECP256K1Hash) {
    println!("rollfoward");
}
pub fn get_block_hash_by_id(_id: u64) -> SECP256K1Hash {
    println!("get_block_hash_by_id");
    [0; 32]
}
pub fn get_latest_block() -> SECP256K1Hash {
    println!("get_latest_block");
    [0; 32]
}
pub fn get_last_block_in_epoch() -> SECP256K1Hash {
    println!("get_last_block_in_epoch");
    [0; 32]
}

#[cfg(test)]
mod test {

    #[test]
    fn test_longest_chain_queue() {
        assert!(true);
    }
}
