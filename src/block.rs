//use std::stream::Stream;
//use futures::io::AsyncRead;

#[derive(Debug)]
pub struct Block {
    
}

impl Block {
    fn new() -> Block {
        Block {}
    }
    fn deserialize(stream: AsyncRead) -> Block {
        Block {}
    }
    fn serialize() -> AsyncRead {
        let retArr: [u8] = [];
        
        retArr
    }
    fn validate()
    fn addTransaction(Transaction)
    fn txCount() -> int
    fn getTransactionByHash(hash) -> Transaction
    fn getTransactionById(int id) -> Transaction
    fn getTransactions() -> [Transaction]
    fn getHash() -> String
    fn hasTransaction(String hash) -> bool
    fn attemptToFindDifficultHash() -> Option<String>
}

// Module std::stream

#[cfg(test)]
mod test {
    #[test]
    fn test_new() {
        assert!(false);
    }
}

