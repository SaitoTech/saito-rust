use crate::slip::Slip;
use crate::transaction::Transaction;
use std::collections::HashMap;

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
        slip::{Slip},
    };
    use std::collections::HashMap;

    #[test]
    fn shashmap_test() {
        let shashmap = Shashmap::new();
        assert_eq!(shashmap.shashmap, HashMap::new());
    }

}


