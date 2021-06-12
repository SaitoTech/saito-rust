use crate::crypto::Sha256Hash;

include!(concat!(env!("OUT_DIR"), "/constants.rs"));

// The epoch should be stored in a ring. The location pointer is the index of the latest block.
// When the length of the blockchain begins to exceed 2x epoch length new blocks will begin to
// overwrite older blocks.
// the top_location + length tells us where valid data is.

const RING_BUFFER_LENGTH: u64 = 2 * EPOCH_LENGTH;

#[derive(Debug, Clone)]
pub struct LongestChainQueue {
    /// This will hold a ring of blocks by block_id, but index is not the block_id.
    epoch_ring_array: [Sha256Hash; RING_BUFFER_LENGTH as usize],
    /// Longest Chain total length, i.e. the latest_block's id
    longest_chain_length: u64,
    /// like a stack pointer, this points to the latest_block
    epoch_ring_top_location: u64,
    /// This should either be the length of the valid data or 2x EPOCH_LENGTH(RING_BUFFER_LENGTH)
    /// When we roll back, some data simply becomes invalid and this number decreases
    epoch_ring_length: u64,
}

impl LongestChainQueue {
    /// Create new `LongestChainQueue`
    pub fn new() -> Self {
        LongestChainQueue {
            epoch_ring_array: [[0; 32]; RING_BUFFER_LENGTH as usize],
            longest_chain_length: 0,
            epoch_ring_top_location: RING_BUFFER_LENGTH - 1, // equivalent to -1, but we have a u64
            epoch_ring_length: 0,
        }
    }
    /// Roll back the longest chain by 1.
    /// We don't actually remove any data, we just assume that only data going back
    /// epoch_ring_length from epoch_ring_top_location is valid(with rollover at 0).
    /// i.e. epoch_ring_length keeps track of how much data is valid.
    pub fn roll_back(&mut self) -> Sha256Hash {
        if self.longest_chain_length == 0 {
            panic!("The longest chain is already 0, we cannot rollback!");
        }
        if self.epoch_ring_length == 0 {
            panic!("We've already rolled back, we can't roll back any farther!");
        }
        self.longest_chain_length -= 1;
        if self.epoch_ring_top_location == 0 {
            self.epoch_ring_top_location = RING_BUFFER_LENGTH - 1;
        } else {
            self.epoch_ring_top_location -= 1;
        }
        self.epoch_ring_length -= 1;
        // We should be safe to just unwrap here instead of returning and Option because
        // we've already done a check about to make sure the chain length is not 0
        //println!("roll_back {:?}", self.latest_block_hash().unwrap());
        self.latest_block_hash().unwrap()
    }
    /// Roll forward the longest chain by 1.
    /// If the length of this has exceeded RING_BUFFER_LENGTH, we simply start over again at 0.
    /// If the length epoch_ring_length exceeds RING_BUFFER_LENGTH, we are overwriting data and
    /// therefore cap epoch_ring_length at RING_BUFFER_LENGTH.
    pub fn roll_forward(&mut self, new_block_hash: Sha256Hash) {
        //println!("roll_forward {:?}", new_block_hash);
        self.longest_chain_length += 1;
        self.epoch_ring_top_location += 1;
        self.epoch_ring_top_location = self.epoch_ring_top_location % RING_BUFFER_LENGTH;
        self.epoch_ring_length += 1;
        self.epoch_ring_length = std::cmp::min(self.epoch_ring_length, RING_BUFFER_LENGTH);
        self.epoch_ring_array[self.epoch_ring_top_location as usize] = new_block_hash;
    }

    pub fn block_hash_by_id(&self, id: u64) -> Sha256Hash {
        if id + 1 > self.longest_chain_length {
            panic!("The block id is greater than the latest block id");
        }
        if self.longest_chain_length - id > self.epoch_ring_length {
            panic!(
                "The block id has fallen off the longest chain ring buffer and cannot be retrieved"
            );
        }
        // The index should be valid as long as the previous check passed
        // We calculate how far back the block is: self.longest_chain_length - id
        // We substract this from the latest_block pointer self.epoch_ring_top_location - how_far_back
        // We then might be less than 0, so we mod with RING_BUFFER_LENGTH.
        let how_far_back: i64 = self.longest_chain_length as i64 - 1 - id as i64;
        let mut index = self.epoch_ring_top_location as i64 - how_far_back;
        if index < 0 {
            // % in rust is a remainder operator, not a modulo, so we have to do this instead...
            index += RING_BUFFER_LENGTH as i64;
        }
        self.epoch_ring_array[index as usize]
    }

    pub fn latest_block_id(&self) -> Option<u64> {
        if self.longest_chain_length == 0 {
            None
        } else {
            Some(self.longest_chain_length - 1)
        }
    }

    pub fn latest_block_hash(&self) -> Option<Sha256Hash> {
        if self.longest_chain_length <= 0 {
            return None;
        }
        Some(self.epoch_ring_array[self.epoch_ring_top_location as usize])
    }

    pub fn last_block_in_epoch(&self) -> Sha256Hash {
        if self.epoch_ring_length < RING_BUFFER_LENGTH
            || self.epoch_ring_top_location == RING_BUFFER_LENGTH - 1
        {
            self.epoch_ring_array[0]
        } else {
            self.epoch_ring_array[(self.epoch_ring_top_location + 1) as usize]
        }
    }

    pub fn contains_hash_by_block_id(&self, hash: Sha256Hash, block_id: u64) -> bool {
        if block_id + 1 > self.longest_chain_length {
            false
        } else {
            self.block_hash_by_id(block_id) == hash
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::crypto::make_message_from_string;

    #[test]
    fn test_longest_chain_queue() {
        // TODO: paramterize the contructor of longest chain with EPOCH_LENGTH rather than reading it from
        // the global so that the length of these tests runs doesn't have to be dependant on EPOCH_LENGTH
        let ring_buffer_length_test: u64 = 2 * EPOCH_LENGTH;

        let mut longest_chain_queue = LongestChainQueue::new();

        for n in 0..100 {
            longest_chain_queue.roll_forward(make_message_from_string(&n.to_string()));
            assert!(longest_chain_queue
                .contains_hash_by_block_id(make_message_from_string(&n.to_string()), n));
        }
        assert_eq!(longest_chain_queue.latest_block_id().unwrap(), 99);
        assert_eq!(
            longest_chain_queue.latest_block_hash().unwrap(),
            make_message_from_string(&99.to_string())
        );
        assert_eq!(
            longest_chain_queue.block_hash_by_id(0),
            make_message_from_string(&0.to_string())
        );
        assert_eq!(
            longest_chain_queue.block_hash_by_id(99),
            make_message_from_string(&99.to_string())
        );
        println!("Expect to see a panic in stdout here:");
        let result = std::panic::catch_unwind(|| longest_chain_queue.block_hash_by_id(100));
        assert!(result.is_err());
        for n in 100..ring_buffer_length_test {
            longest_chain_queue.roll_forward(make_message_from_string(&n.to_string()));
        }
        assert_eq!(
            longest_chain_queue.block_hash_by_id(0),
            make_message_from_string(&0.to_string())
        );
        longest_chain_queue.roll_forward(make_message_from_string(
            &ring_buffer_length_test.to_string(),
        ));
        assert_eq!(
            longest_chain_queue.latest_block_id().unwrap(),
            ring_buffer_length_test
        );
        assert_eq!(
            longest_chain_queue.latest_block_hash().unwrap(),
            make_message_from_string(&ring_buffer_length_test.to_string())
        );

        println!("Expect to see a panic in stdout here:");
        let result = std::panic::catch_unwind(|| longest_chain_queue.block_hash_by_id(0));
        assert!(result.is_err());
        for _n in 0..101 {
            longest_chain_queue.roll_back();
        }
        assert_eq!(
            longest_chain_queue.latest_block_id().unwrap(),
            ring_buffer_length_test - 101
        );
        assert_eq!(
            longest_chain_queue.latest_block_hash().unwrap(),
            make_message_from_string(&(ring_buffer_length_test - 101).to_string())
        );
        for n in 100..201 {
            longest_chain_queue.roll_forward(make_message_from_string(
                &(ring_buffer_length_test - 200 + n).to_string(),
            ));
        }
        assert_eq!(
            longest_chain_queue.latest_block_id().unwrap(),
            ring_buffer_length_test
        );
        assert_eq!(
            longest_chain_queue.latest_block_hash().unwrap(),
            make_message_from_string(&ring_buffer_length_test.to_string())
        );
        // TODO test last_block_in_epoch()
    }
}
