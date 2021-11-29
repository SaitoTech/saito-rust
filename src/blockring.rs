use crate::block::Block;
use crate::blockchain::GENESIS_PERIOD;
use crate::crypto::SaitoHash;

pub const RING_BUFFER_LENGTH: u64 = 2 * GENESIS_PERIOD;

use log::trace;

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
    lc_pos: Option<usize>, // which idx in the vectors below points to the longest-chain block
    pub block_hashes: Vec<SaitoHash>,
    block_ids: Vec<u64>,
}

impl RingItem {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            lc_pos: None,
            block_hashes: vec![],
            block_ids: vec![],
        }
    }

    pub fn contains_block_hash(&self, hash: SaitoHash) -> bool {
        self.block_hashes.iter().any(|&i| i == hash)
    }

    pub fn add_block(&mut self, block_id: u64, hash: SaitoHash) {
        self.block_hashes.push(hash);
        self.block_ids.push(block_id);
    }

    pub fn delete_block(&mut self, block_id: u64, hash: SaitoHash) {
        let mut new_block_hashes: Vec<SaitoHash> = vec![];
        let mut new_block_ids: Vec<u64> = vec![];
        let mut idx_loop = 0;
        let mut new_lc_pos = Some(0);

        for i in 0..self.block_ids.len() {
            if self.block_ids[i] == block_id && self.block_hashes[i] == hash {
            } else {
                new_block_hashes.push(self.block_hashes[i]);
                new_block_ids.push(self.block_ids[i]);
                if self.lc_pos == Some(i) {
                    new_lc_pos = Some(idx_loop);
                }
                idx_loop += 1;
            }
        }

        self.block_hashes = new_block_hashes;
        self.block_ids = new_block_ids;
        self.lc_pos = new_lc_pos;
    }

    pub fn on_chain_reorganization(&mut self, hash: SaitoHash, lc: bool) -> bool {
        if !lc {
            self.lc_pos = None;
        } else {
            self.lc_pos = self.block_hashes.iter().position(|b_hash| b_hash == &hash);
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
    pub block_ring: Vec<RingItem>,
    /// a ring of blocks, index is not the block_id.
    block_ring_lc_pos: Option<usize>,
}

impl BlockRing {
    /// Create new `BlockRing`
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        //
        // initialize the block-ring
        //
        let mut init_block_ring: Vec<RingItem> = vec![];
        for _i in 0..RING_BUFFER_LENGTH {
            init_block_ring.push(RingItem::new());
        }

        BlockRing {
            block_ring: init_block_ring,
            block_ring_lc_pos: None,
        }
    }

    pub fn print_lc(&self) {
        for i in 0..GENESIS_PERIOD {
            if !self.block_ring[(i as usize)].block_hashes.is_empty() {
                trace!(
                    "Block {:?}: {:?}",
                    i,
                    self.get_longest_chain_block_hash_by_block_id(i)
                );
            }
        }
    }

    pub fn contains_block_hash_at_block_id(&self, block_id: u64, block_hash: SaitoHash) -> bool {
        let insert_pos = block_id % RING_BUFFER_LENGTH;
        self.block_ring[(insert_pos as usize)].contains_block_hash(block_hash)
    }

    pub fn is_empty(&self) -> bool {
        return self.block_ring_lc_pos.is_none();
    }

    pub fn add_block(&mut self, block: &Block) {
        let insert_pos = block.get_id() % RING_BUFFER_LENGTH;
        self.block_ring[(insert_pos as usize)].add_block(block.get_id(), block.get_hash());
    }

    pub fn delete_block(&mut self, block_id: u64, block_hash: SaitoHash) {
        let insert_pos = block_id % RING_BUFFER_LENGTH;
        self.block_ring[(insert_pos as usize)].delete_block(block_id, block_hash);
    }

    pub fn get_block_hashes_at_block_id(&mut self, block_id: u64) -> Vec<SaitoHash> {
        let insert_pos = block_id % RING_BUFFER_LENGTH;
        let mut v: Vec<SaitoHash> = vec![];
        for i in 00..self.block_ring[(insert_pos as usize)].block_hashes.len() {
            if self.block_ring[(insert_pos as usize)].block_ids[i] == block_id {
                v.push(self.block_ring[(insert_pos as usize)].block_hashes[i].clone());
            }
        }
        v
    }

    pub fn on_chain_reorganization(&mut self, block_id: u64, hash: SaitoHash, lc: bool) -> bool {
        let insert_pos = block_id % RING_BUFFER_LENGTH;
        if !self.block_ring[(insert_pos as usize)].on_chain_reorganization(hash, lc) {
            return false;
        }
        if lc {
            self.block_ring_lc_pos = Some(insert_pos as usize);
        } else {
            //
            // if we are unsetting the longest-chain, we automatically
            // roll backwards and set the longest-chain to the previous
            // position if available. this adds some complexity to unwinding
            // the chain but should ensure that in most situations there is
            // always a known longest-chain position.
            //
            if let Some(block_ring_lc_pos) = self.block_ring_lc_pos {
                if block_ring_lc_pos == insert_pos as usize {
                    let previous_block_idx;

                    if block_ring_lc_pos > 0 {
                        previous_block_idx = block_ring_lc_pos - 1;
                    } else {
                        previous_block_idx = RING_BUFFER_LENGTH as usize - 1;
                    }

                    // reset to lc_pos to unknown
                    self.block_ring_lc_pos = None;

                    // but try to find it
                    // let previous_block_idx_lc_pos = self.block_ring[previous_block_idx as usize].lc_pos;
                    if let Some(previous_block_idx_lc_pos) =
                        self.block_ring[previous_block_idx as usize].lc_pos
                    {
                        if self.block_ring[previous_block_idx].block_ids.len()
                            > previous_block_idx_lc_pos
                        {
                            if self.block_ring[previous_block_idx].block_ids
                                [previous_block_idx_lc_pos]
                                == block_id - 1
                            {
                                self.block_ring_lc_pos = Some(previous_block_idx);
                            }
                        }
                    }
                }
            }
        }
        true
    }

    pub fn get_longest_chain_block_hash_by_block_id(&self, id: u64) -> SaitoHash {
        let insert_pos = (id % RING_BUFFER_LENGTH) as usize;
        match self.block_ring[insert_pos].lc_pos {
            Some(lc_pos) => self.block_ring[insert_pos].block_hashes[lc_pos],
            None => [0; 32],
        }
    }

    pub fn get_latest_block_hash(&self) -> SaitoHash {
        match self.block_ring_lc_pos {
            Some(block_ring_lc_pos) => match self.block_ring[block_ring_lc_pos].lc_pos {
                Some(lc_pos) => self.block_ring[block_ring_lc_pos].block_hashes[lc_pos],
                None => [0; 32],
            },
            None => [0; 32],
        }
    }

    pub fn get_latest_block_id(&self) -> u64 {
        match self.block_ring_lc_pos {
            Some(block_ring_lc_pos) => match self.block_ring[block_ring_lc_pos].lc_pos {
                Some(lc_pos) => self.block_ring[block_ring_lc_pos].block_ids[lc_pos],
                None => 0,
            },
            None => 0,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::blockchain::Blockchain;
    use crate::test_utilities::test_manager::TestManager;
    use crate::time::create_timestamp;
    use crate::wallet::Wallet;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    use super::*;

    #[tokio::test]
    #[serial_test::serial]
    // winding / unwinding updates blockring view of longest chain
    async fn blockring_manual_reorganization_test() {
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let test_manager = TestManager::new(blockchain_lock.clone(), wallet_lock.clone());
        let mut blockring = BlockRing::new();

        let current_timestamp = create_timestamp();

        // BLOCK 1
        let mut block1 = test_manager
            .generate_block_and_metadata([0; 32], current_timestamp, 3, 0, false, vec![])
            .await;
        block1.set_id(1);

        // BLOCK 2
        let mut block2 = test_manager
            .generate_block_and_metadata(
                block1.get_hash(),
                current_timestamp + 120000,
                0,
                1,
                false,
                vec![],
            )
            .await;
        block2.set_id(2);

        // BLOCK 3
        let mut block3 = test_manager
            .generate_block_and_metadata(
                block2.get_hash(),
                current_timestamp + 240000,
                0,
                1,
                false,
                vec![],
            )
            .await;
        block3.set_id(3);

        // BLOCK 4
        let mut block4 = test_manager
            .generate_block_and_metadata(
                block3.get_hash(),
                current_timestamp + 360000,
                0,
                1,
                false,
                vec![],
            )
            .await;
        block4.set_id(4);

        // BLOCK 5
        let mut block5 = test_manager
            .generate_block_and_metadata(
                block4.get_hash(),
                current_timestamp + 480000,
                0,
                1,
                false,
                vec![],
            )
            .await;
        block5.set_id(5);

        blockring.add_block(&block1);
        blockring.add_block(&block2);
        blockring.add_block(&block3);
        blockring.add_block(&block4);
        blockring.add_block(&block5);

        // do we contain these block hashes?
        assert_eq!(
            blockring.contains_block_hash_at_block_id(1, block1.get_hash()),
            true
        );
        assert_eq!(
            blockring.contains_block_hash_at_block_id(2, block2.get_hash()),
            true
        );
        assert_eq!(
            blockring.contains_block_hash_at_block_id(3, block3.get_hash()),
            true
        );
        assert_eq!(
            blockring.contains_block_hash_at_block_id(4, block4.get_hash()),
            true
        );
        assert_eq!(
            blockring.contains_block_hash_at_block_id(5, block5.get_hash()),
            true
        );
        assert_eq!(
            blockring.contains_block_hash_at_block_id(2, block4.get_hash()),
            false
        );

        // reorganize longest chain
        blockring.on_chain_reorganization(1, block1.get_hash(), true);
        assert_eq!(blockring.get_latest_block_id(), 1);
        blockring.on_chain_reorganization(2, block2.get_hash(), true);
        assert_eq!(blockring.get_latest_block_id(), 2);
        blockring.on_chain_reorganization(3, block3.get_hash(), true);
        assert_eq!(blockring.get_latest_block_id(), 3);
        blockring.on_chain_reorganization(4, block4.get_hash(), true);
        assert_eq!(blockring.get_latest_block_id(), 4);
        blockring.on_chain_reorganization(5, block5.get_hash(), false);
        assert_eq!(blockring.get_latest_block_id(), 4);
        blockring.on_chain_reorganization(4, block4.get_hash(), false);
        assert_eq!(blockring.get_latest_block_id(), 3);
        blockring.on_chain_reorganization(3, block3.get_hash(), false);
        assert_eq!(blockring.get_latest_block_id(), 2);

        // reorg in the wrong block_id location, should not change
        blockring.on_chain_reorganization(532, block5.get_hash(), false);
        assert_eq!(blockring.get_latest_block_id(), 2);

        // double reorg in correct and should be fine still
        blockring.on_chain_reorganization(2, block2.get_hash(), true);
        assert_eq!(blockring.get_latest_block_id(), 2);
    }

    #[tokio::test]
    #[serial_test::serial]
    // adding blocks to blockchain wind / unwind blockring view of longest chain
    async fn blockring_automatic_reorganization_test() {
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let test_manager = TestManager::new(blockchain_lock.clone(), wallet_lock.clone());

        let current_timestamp = create_timestamp();

        // BLOCK 1
        let block1 = test_manager
            .generate_block_and_metadata([0; 32], current_timestamp, 3, 0, false, vec![])
            .await;
        let block1_hash = block1.get_hash();
        Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block1).await;

        // BLOCK 2
        let block2 = test_manager
            .generate_block_and_metadata(
                block1_hash,
                current_timestamp + 120000,
                0,
                1,
                false,
                vec![],
            )
            .await;
        let block2_hash = block2.get_hash();
        Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block2).await;

        // BLOCK 3
        let block3 = test_manager
            .generate_block_and_metadata(
                block2_hash,
                current_timestamp + 240000,
                0,
                1,
                false,
                vec![],
            )
            .await;
        let block3_hash = block3.get_hash();
        Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block3).await;

        // BLOCK 4
        let block4 = test_manager
            .generate_block_and_metadata(
                block3_hash,
                current_timestamp + 360000,
                0,
                1,
                false,
                vec![],
            )
            .await;
        let block4_hash = block4.get_hash();
        Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block4).await;

        // BLOCK 5
        let block5 = test_manager
            .generate_block_and_metadata(
                block4_hash,
                current_timestamp + 480000,
                0,
                1,
                false,
                vec![],
            )
            .await;
        let block5_hash = block5.get_hash();
        Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block5).await;

        let mut blockchain = blockchain_lock.write().await;

        // do we contain these block hashes?
        assert_eq!(
            blockchain
                .blockring
                .contains_block_hash_at_block_id(1, block1_hash),
            true
        );
        assert_eq!(
            blockchain
                .blockring
                .contains_block_hash_at_block_id(2, block2_hash),
            true
        );
        assert_eq!(
            blockchain
                .blockring
                .contains_block_hash_at_block_id(3, block3_hash),
            true
        );
        assert_eq!(
            blockchain
                .blockring
                .contains_block_hash_at_block_id(4, block4_hash),
            true
        );
        assert_eq!(
            blockchain
                .blockring
                .contains_block_hash_at_block_id(5, block5_hash),
            true
        );
        assert_eq!(
            blockchain
                .blockring
                .contains_block_hash_at_block_id(2, block4_hash),
            false
        );

        // reorganize longest chain
        assert_ne!(blockchain.blockring.get_latest_block_id(), 1);
        assert_ne!(blockchain.blockring.get_latest_block_id(), 2);
        assert_ne!(blockchain.blockring.get_latest_block_id(), 3);
        assert_ne!(blockchain.blockring.get_latest_block_id(), 4);
        assert_eq!(blockchain.blockring.get_latest_block_id(), 5);
        assert_eq!(blockchain.blockring.get_latest_block_hash(), block5_hash);

        // reorg in the wrong block_id location, should not change
        blockchain
            .blockring
            .on_chain_reorganization(532, block5_hash, false);
        assert_ne!(blockchain.blockring.get_latest_block_id(), 2);

        blockchain
            .blockring
            .on_chain_reorganization(5, block5_hash, false);
        assert_eq!(blockchain.blockring.get_latest_block_id(), 4);
        assert_eq!(blockchain.blockring.get_latest_block_hash(), block4_hash);
    }

    #[tokio::test]
    #[serial_test::serial]
    // confirm adding block changes nothing until on_chain_reorg
    async fn blockring_add_block_test() {
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let test_manager = TestManager::new(blockchain_lock.clone(), wallet_lock.clone());

        let current_timestamp = create_timestamp();

        let mut blockring = BlockRing::new();
        assert_eq!(0, blockring.get_latest_block_id());

        // BLOCK 1
        let block1 = test_manager
            .generate_block_and_metadata([0; 32], current_timestamp, 3, 0, false, vec![])
            .await;
        let block1_hash = block1.get_hash();
        blockring.add_block(&block1);
        Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block1).await;

        // BLOCK 2
        let block2 = test_manager
            .generate_block_and_metadata(
                block1_hash,
                current_timestamp + 120000,
                0,
                1,
                false,
                vec![],
            )
            .await;
        let block2_hash = block2.get_hash();
        blockring.add_block(&block2);
        Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block2).await;

        assert_eq!(0, blockring.get_latest_block_id());
        assert_eq!([0; 32], blockring.get_latest_block_hash());
        assert_eq!(
            [0; 32],
            blockring.get_longest_chain_block_hash_by_block_id(0)
        );

        blockring.on_chain_reorganization(1, block1_hash, true);

        assert_eq!(1, blockring.get_latest_block_id());
        assert_eq!(block1_hash, blockring.get_latest_block_hash());
        assert_eq!(
            [0; 32],
            blockring.get_longest_chain_block_hash_by_block_id(0)
        );

        blockring.on_chain_reorganization(1, block1_hash, false);

        assert_eq!(0, blockring.get_latest_block_id());
        assert_eq!([0; 32], blockring.get_latest_block_hash());
        assert_eq!(
            [0; 32],
            blockring.get_longest_chain_block_hash_by_block_id(0)
        );

        blockring.on_chain_reorganization(1, block1_hash, true);
        blockring.on_chain_reorganization(2, block2_hash, true);

        assert_eq!(2, blockring.get_latest_block_id());
        assert_eq!(block2_hash, blockring.get_latest_block_hash());
        assert_eq!(
            [0; 32],
            blockring.get_longest_chain_block_hash_by_block_id(3)
        );
    }
}
