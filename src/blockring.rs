use crate::block::Block;
use crate::crypto::SaitoHash;

const EPOCH_LENGTH: u64 = 21000;
const RING_BUFFER_LENGTH: u64 = 2 * EPOCH_LENGTH;

//
// This is an index with shorthand information on the block_ids and hashes of the blocks
// in the longest-chain.
//
// The BlockRing is a fixed size Vector which can be made contiguous and theoretically
// made available for fast-access through a slice with the same lifetime as the vector
// itself.
//
#[derive(Debug)]
pub struct RingItem {
    lc_pos: usize, // which idx in the vectors below points to the longest-chain block
    block_hashes: Vec<SaitoHash>,
    block_ids: Vec<u64>,
}

impl RingItem {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            lc_pos: usize::MAX,
            block_hashes: vec![],
            block_ids: vec![],
        }
    }

    pub fn contains_block_hash(&mut self, hash: SaitoHash) -> bool {
        self.block_hashes.iter().any(|&i| i == hash)
    }

    pub fn add_block(&mut self, block_id: u64, hash: SaitoHash) {
        self.block_hashes.push(hash);
        self.block_ids.push(block_id);
    }

    pub fn on_chain_reorganization(&mut self, hash: SaitoHash, lc: bool) -> bool {
        if !lc {
            self.lc_pos = usize::MAX;
        } else {
            //
            // update new longest-chain
            //
            for i in 0..self.block_hashes.len() {
                if self.block_hashes[i] == hash {
                    self.lc_pos = i;
                }
            }

            //
            // this hash does not exist
            //
            if self.block_ids.len() < self.lc_pos {
                return false;
            }

            //
            // remove any old indices
            //
            let current_block_id = self.block_ids[self.lc_pos];

            let mut new_block_hashes: Vec<SaitoHash> = vec![];
            let mut new_block_ids: Vec<u64> = vec![];

            for i in 0..self.block_ids.len() {
                if self.block_ids[i] < current_block_id {
                    self.lc_pos = i;
                } else {
                    new_block_hashes.push(self.block_hashes[i]);
                    new_block_ids.push(self.block_ids[i]);
                }
            }

            self.block_hashes = new_block_hashes;
            self.block_ids = new_block_ids;
        }

        true
    }
}

//
// TODO - we can optimize this.
//
// The Blockchain Index is a self-contained data structure
//
#[derive(Debug)]
pub struct BlockRing {
    //
    // each ring_item is a point on our blockchain
    //
    // include Slice-VecDeque and have a slice that points to
    // contiguous entries for rapid lookups, inserts and updates?
    //
    block_ring: Vec<RingItem>,
    /// a ring of blocks, index is not the block_id.
    block_ring_lc_pos: usize,
}

impl BlockRing {
    /// Create new `BlockRing`
    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> Self {
        //
        // initialize the block-ring
        //
        let mut init_block_ring: Vec<RingItem> = vec![];
        for _i in 0..EPOCH_LENGTH {
            init_block_ring.push(RingItem::new());
        }

        BlockRing {
            block_ring: init_block_ring,
            block_ring_lc_pos: usize::MAX,
        }
    }

    pub fn print_lc(&self) {
        for i in 0..EPOCH_LENGTH {
            if !self.block_ring[(i as usize)].block_hashes.is_empty() {
                println!(
                    "Block {:?}: {:?}",
                    i,
                    self.get_longest_chain_block_hash_by_block_id(i)
                );
            }
        }
    }

    pub fn contains_block_hash_at_block_id(
        &mut self,
        block_id: u64,
        block_hash: SaitoHash,
    ) -> bool {
        let insert_pos = block_id % RING_BUFFER_LENGTH;
        self.block_ring[(insert_pos as usize)].contains_block_hash(block_hash)
    }

    pub fn add_block(&mut self, block: &Block) {
        let insert_pos = block.get_id() % RING_BUFFER_LENGTH;
        self.block_ring[(insert_pos as usize)].add_block(block.get_id(), block.get_hash());
    }

    pub fn on_chain_reorganization(&mut self, block_id: u64, hash: SaitoHash, lc: bool) -> bool {
        let insert_pos = block_id % RING_BUFFER_LENGTH;
        if !self.block_ring[(insert_pos as usize)].on_chain_reorganization(hash, lc) {
            return false;
        }
        if lc {
            self.block_ring_lc_pos = insert_pos as usize;
        } else {
            //
            // only adjust longest_chain if this is it
            //
            if self.block_ring_lc_pos == insert_pos as usize {
                let previous_block_idx = self.block_ring_lc_pos - 1;

                // reset to lc_pos to unknown
                self.block_ring_lc_pos = usize::MAX;

                // but try to find it
                let previous_block_idx_lc_pos = self.block_ring[previous_block_idx as usize].lc_pos;
                if previous_block_idx_lc_pos != usize::MAX {
                    if self.block_ring[previous_block_idx].block_ids.len()
                        > previous_block_idx_lc_pos
                    {
                        if self.block_ring[previous_block_idx].block_ids[previous_block_idx_lc_pos]
                            == block_id - 1
                        {
                            self.block_ring_lc_pos = previous_block_idx;
                        }
                    }
                }
            }
        }
        true
    }

    pub fn get_longest_chain_block_hash_by_block_id(&self, id: u64) -> SaitoHash {
        let insert_pos = id % RING_BUFFER_LENGTH;
        let lc_pos = self.block_ring[(insert_pos as usize)].lc_pos;
        if lc_pos != usize::MAX {
            self.block_ring[(insert_pos as usize)].block_hashes[lc_pos]
        } else {
            [0; 32]
        }
    }

    pub fn get_longest_chain_block_hash(&self) -> SaitoHash {
        if self.block_ring_lc_pos == usize::MAX
            || self.block_ring[self.block_ring_lc_pos].lc_pos == usize::MAX
        {
            [0; 32]
        } else {
            let lc_pos = self.block_ring[self.block_ring_lc_pos].lc_pos;
            if lc_pos != usize::MAX {
                self.block_ring[self.block_ring_lc_pos].block_hashes[lc_pos]
            } else {
                [0; 32]
            }
        }
    }

    pub fn get_longest_chain_block_id(&self) -> u64 {
        if self.block_ring_lc_pos == usize::MAX
            || self.block_ring[self.block_ring_lc_pos].lc_pos == usize::MAX
        {
            0
        } else {
            let lc_pos = self.block_ring[self.block_ring_lc_pos].lc_pos;
            if lc_pos != usize::MAX {
                self.block_ring[self.block_ring_lc_pos].block_ids[lc_pos]
            } else {
                0
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::test_utilities::mocks::make_mock_block;

    use super::*;
    #[test]
    fn add_block_test() {
        let mut blockring = BlockRing::new();

        //
        // Good Blocks
        //
        let block_1 = make_mock_block([0; 32], 1);
        let block_2 = make_mock_block(block_1.get_hash(), 2);
        let block_3 = make_mock_block(block_2.get_hash(), 3);
        let block_4 = make_mock_block(block_3.get_hash(), 4);
        let block_3_2 = make_mock_block(block_2.get_hash(), 3);
        let block_4_2 = make_mock_block(block_3.get_hash(), 4);
        let block_5_2 = make_mock_block(block_4.get_hash(), 5);

        blockring.add_block(&block_1);
        blockring.add_block(&block_2);
        blockring.add_block(&block_3);
        blockring.add_block(&block_4);
        blockring.add_block(&block_3_2);
        blockring.add_block(&block_4_2);
        blockring.add_block(&block_5_2);

        // do we contain these block hashes?
        assert_eq!(
            blockring.contains_block_hash_at_block_id(1, block_1.get_hash()),
            true
        );
        assert_eq!(
            blockring.contains_block_hash_at_block_id(2, block_2.get_hash()),
            true
        );
        assert_eq!(
            blockring.contains_block_hash_at_block_id(3, block_3.get_hash()),
            true
        );
        assert_eq!(
            blockring.contains_block_hash_at_block_id(4, block_4.get_hash()),
            true
        );
        assert_eq!(
            blockring.contains_block_hash_at_block_id(3, block_3_2.get_hash()),
            true
        );
        assert_eq!(
            blockring.contains_block_hash_at_block_id(4, block_4_2.get_hash()),
            true
        );
        assert_eq!(
            blockring.contains_block_hash_at_block_id(5, block_5_2.get_hash()),
            true
        );

        // reorganize longest chain
        blockring.on_chain_reorganization(1, block_1.get_hash(), true);
        blockring.on_chain_reorganization(2, block_2.get_hash(), true);
        blockring.on_chain_reorganization(3, block_3.get_hash(), true);
        blockring.on_chain_reorganization(4, block_4.get_hash(), true);
        blockring.on_chain_reorganization(4, block_4.get_hash(), false);
        blockring.on_chain_reorganization(3, block_3.get_hash(), false);
        blockring.on_chain_reorganization(3, block_3_2.get_hash(), true);
        blockring.on_chain_reorganization(4, block_4_2.get_hash(), true);
        blockring.on_chain_reorganization(5, block_5_2.get_hash(), true);

        assert_eq!(blockring.get_longest_chain_block_id(), 5);
        // reorg in the wrong location
        blockring.on_chain_reorganization(532, block_5_2.get_hash(), false);
        assert_eq!(blockring.get_longest_chain_block_id(), 5);

        blockring.on_chain_reorganization(5, block_5_2.get_hash(), true);
        assert_eq!(blockring.get_longest_chain_block_id(), 5);
    }
}
