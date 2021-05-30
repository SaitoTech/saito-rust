use crate::slip::Slip;
use crate::transaction::Transaction;
use std::collections::HashMap;

/// A hashmap storing the byte arrays of `Slip`s as keys
/// with the `Block` ids as values. This is used to enforce when
/// `Slip`s have been spent in the network
#[derive(Debug, Clone)]
pub struct Shashmap {
    shashmap: HashMap<Slip, i64>,
}

impl Shashmap {
    /// Create new `Shashmap`
    pub fn new() -> Self {
        Shashmap {
            shashmap: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{
        keypair::Keypair,
        slip::{Slip},
    };
    use std::collections::HashMap;

    #[test]
    fn shashmap_test() {
        let shashmap = Shashmap::new();
        assert_eq!(shashmap.shashmap, HashMap::new());
    }

}


