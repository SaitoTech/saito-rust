# Unit/Module Interfaces

The following is our first draft of the responsibilities and separation of concerns for the Saito Rust architecture.


### Main

Main process loop, reads flags from command line, inits things, and starts running all the things

* main()

### Tokio Channels

Modules will publish events like blockCandidateFound, DifficultHashFound, TxSignedForDispatch which other modules can listen to and react to appropriately through Tokio Async Runtime.

### Slip

* new(SlipType type, String toAddress, String fromAddress, BigInt amount) -> Slip
* setIds(int bid, int tid, int sid)
* getAmt() -> int
* setAmt(int amt)
* setBroadcastType(broadcast_type: SlipType)
* setSpentStatus(spent_status: SlipSpentStatus)
* returnAddress() -> PublicKey
* returnSignatureSource() -> Vec<u8>
* setBlockHash(ByteArray block_hash)

### Hop

* new(to: PublicKey, from: PublicKey, sig: Signature)

### Transaction

* new(type TxTypeEnum, sig String, slips [Slip])
* deserialize(Stream or String)
* serialize() -> Stream or String
* validate()
* getType()
* addSlip(Slip)
* getSlips -> [Slip]
* pushToPath(Hop hop)
* createRebroadcastTxFrom() -> Transaction

### Block

* new()
* deserialize(Stream)
* serialize() -> Stream
* validate()
* addTransaction(Transaction)
* txCount() -> int
* getTransactionByHash(hash) -> Transaction
* getTransactionById(int id) -> Transaction
* getTransactions() -> [Transaction]
* getHash() -> String
* hasTransaction(String hash) -> bool
* attemptToFindDifficultHash() -> Option<String>

### Blockchain

Responsible for storing the blockchain to disk, interacting with the UTXO set and blocks/tx in the blockchain

* new()
* getLatest()
* getBlockById(int id) -> Block
* getBlockByHash(String hash) -> Block
* addBlock(Block block, int parentId)
* getBlockParent(Block block) -> Block
* *Event: rollBackBlock(Block block)*
* *Event: rollForwardBlock(Block block)*


### UTXOSet

All the unspent outputs

* getSlip(int blockID, int txID, int slipID) -> Slip
* addSlip(int blockID, int txID, int slipID, Slip slip)
* removeSlip(int blockID, int txID, int slipID)

### SaitoConsensusGoldenTicketHasher

Responsible for finding golden tickets

* *Event: difficultHashFound(Block block)*
* *listens for blockchain::rollForwardBlock*

### SaitoConsensusBlockProducer

* startProducingBlocks()
* *Event: blockProduced(Block block)*

### SaitoConsensusRouter

Responsible for sending tx which are not yet in a block, i.e. doing Routing Consensus work

* maybeRouteTransaction(Transaction tx)

### SaitoConsensusValidator

Validates transactions and blocks before adding to chain

* validateTx(Transaction tx)
* validateBlock(Block blk)

### BurnFeeCalculator

Calculates work based on the state of the blockchain

* calculateWorkNeeded(int prevts, int current_time, float burn_fee)
* getBurnFee(Block prevblk, Block blk)

### Mempool

An in-memory store of all the known transactions which are not yet in a block

* hasTx(Transaction tx)
* addTx(Transaction tx)
* removeTx(String txhash)
* getHighestPayingTransaction()
* getHighestPayingTransactions(int bytecount)

### PeerPool

Responsible for managing peer connections

* init()
* addPeer(Peer peer)
* dropPeer(Peer peer)
* getAllPeers()
* routeTxToAllPeers(Transaction tx)

### Peer

A Class representing a peer

* routeTxToPeer(Transaction tx)
* sendTxToPeer(Transaction tx)
* sendBlockToPeer(Block block)

### KeypairStore

Initializes(generates) keypairs on first startup, encrypts private keys to disk. Can also be provided with a command line switch a keypair file.

* initialize()
* getKeypair() - Keypair //I’m not sure if we want to allow this ti manage multiple accounts/keypairs

### Keypair

* sign(String message) -> String
* getPubkey() -> String  //Is there a difference between address and publickey in Saito-lite? Let’s just make them one thing
* getPrivateKey() -> String

### Crypto

Static crypto functions like in crypto.js

* generateKeypair() -> Keypair
* verifyMessage(String message) -> bool
* isPublicKey(String key) -> bool

## Policy Classes:

There are often subjective choices for a node to make, e.g. whether to connect to a peer, whether to accept blocks from a peer, whether to route transactions, and who to route transactions to. The purposes of the following classes are to provide policy/strategy patterns around these functions. I.e. command line flags may choose different Policy Implemenations at startup or users may even opt to write their own custom policies.

We’re not certain what functions should be in the concrete policies, but this will become clearer as we are writing code and as the network gets used.

### PeerPolicy

* static getPolicy() -> ConcretePeerPolicy
* setPolicy<T:ConcretePeerPolicy>(policy: T)

### ConcretePeerPolicy

* shouldBlockPeerByIP(Peer peer) -> bool // may need to store blacklisted peers to disk and interact with system to block at the http frontend(e.g. nginx)
* shouldBlockPeerByAddress(Peer peer) // may need to store blacklisted peers to disk
* shoudAcceptAndValidateByFromPeer(Peer peer, Block block) -> bool
* getInitialPeers() -> [peer]

Example 1: NginxBlacklistByIPPolicy is a ConcretePeerPolicy. Might cache blacklisted IPs to disk and then configure nginx on the system to ignore those IPs

Example 2: MySqlBlacklistByAddressPolicy is a ConcretePeerPolicy. Caches blacklisted Addresses with some metadata to rows in a MySQL DB.

### ConsensusPolicy

* static getPolicy() -> ConcreteConsensusPolicy
* setPolicy<T:ConcreteConsensusPolicy>(policy: T)

### ConcreteConsensusPolicy

shouldPeerBeRoutedTransaction(Option<Peer> peer, Option<Transaction> tx) -> bool
shouldTryProduceNextBlock() -> bool