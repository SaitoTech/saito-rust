use crate::crypto::Sha256Hash;

// TODO put these sort of consts into a single location.
pub const EPOCH_LENGTH: usize = 30_000;

const RING_BUFFER_LENGTH: usize = 2 * EPOCH_LENGTH;

// TODO Put this into crypto and use it everywhere we have Sha256Hash

#[derive(Debug, Clone)]
pub struct LongestChainQueue {
    pub epoch_ring_array: [Sha256Hash; RING_BUFFER_LENGTH],
    // like a stack pointer
    pub epoch_ring_top_location: usize,
    // This should either be the lenght of the blockchain or 2x EPOCH_LENGTH(RING_BUFFER_LENGTH)
    epoch_ring_length: usize,
}

impl LongestChainQueue {
    /// Create new `LongestChainQueue`
    pub fn new() -> Self {
        LongestChainQueue {
            epoch_ring_array: [[0; 32]; RING_BUFFER_LENGTH],
            epoch_ring_top_location: 0,
            epoch_ring_length: 0,
        }
    }
    pub fn roll_back(&self) -> Sha256Hash {
        println!("rollback");
        [0; 32]
    }
    pub fn roll_forward(&self, _new_block_hash: Sha256Hash) {
        println!("rollfoward");
    }

    pub fn block_hash_by_id(&self, id: u64) -> Sha256Hash {
        self.epoch_ring_array[id as usize & RING_BUFFER_LENGTH]
    }

    pub fn latest_block(&self) -> Option<Sha256Hash> {
        let latest_block_hash = self.epoch_ring_array[self.epoch_ring_top_location];
        if latest_block_hash == [0; 32] {
            None
        } else {
            Some(latest_block_hash)
        }
    }
    pub fn last_block_in_epoch(&self) -> Sha256Hash {
        if self.epoch_ring_length < RING_BUFFER_LENGTH || self.epoch_ring_top_location == RING_BUFFER_LENGTH - 1 {
            self.epoch_ring_array[0]
        } else {
            self.epoch_ring_array[self.epoch_ring_top_location + 1]
        }
    }

    pub fn contains_hash(&self, block_hash: &Sha256Hash) -> bool {
        self.epoch_ring_array.iter().any(|&hash| &hash == block_hash)
    }
}

// The epoch should be stored in a ring. The location pointer is the index of the latest block.
// When the length of the blockchain begins to exceed 2x epoch length new blocks will begin to
// overwrite older blocks. During a rollback those blocks which fell off won't be recovered.
// I don't think we need to store these as Option<Sha256Hash> because the top_location + length
// can tell us where the valid data is.

#[cfg(test)]
mod test {

    #[test]
    fn test_longest_chain_queue() {
        assert!(true);
    }
}
