use crate::crypto::{hash, SaitoHash};

//
// MerkleTreeLayer is a short implementation that uses the default
// Saito hashing algorithm. It is also written to take advantage of
// rayon parallelization.
//
#[derive(PartialEq, Debug, Clone)]
pub struct MerkleTreeLayer {
    left: SaitoHash,
    right: SaitoHash,
    hash: SaitoHash,
    level: u8,
}
impl MerkleTreeLayer {
    pub fn new(left: SaitoHash, right: SaitoHash, level: u8) -> MerkleTreeLayer {
        MerkleTreeLayer {
            left,
            right,
            level,
            hash: [0; 32],
        }
    }

    pub fn hash(&mut self) -> bool {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.left);
        vbytes.extend(&self.right);
        self.hash = hash(&vbytes);
        true
    }

    pub fn get_hash(&self) -> SaitoHash {
        self.hash
    }
}
