var searchIndex = JSON.parse('{\
"saito_rust":{"doc":"Welcome to Saito","t":[0,8,10,10,0,3,11,3,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,0,3,11,11,11,11,11,11,11,11,11,11,11,0,3,11,11,11,11,3,11,11,11,11,11,11,11,11,0,4,13,12,13,12,5,0,3,3,3,3,7,6,6,6,6,6,17,5,5,5,5,0,4,13,13,4,13,13,3,11,11,11,5,0,5,0,4,13,3,11,3,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,0,5,5,3,11,11,3,11,11,11,11,0,4,13,3,11,3,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,0,3,11,11,11,11,6,6,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11],"n":["big_array","BigArray","serialize","deserialize","block","BlockCore","new","Block","new","get_hash","get_lc","get_id","get_timestamp","get_previous_block_hash","get_creator","get_merkle_root","get_signature","get_treasury","get_burnfee","get_difficulty","set_id","set_lc","set_timestamp","set_previous_block_hash","set_creator","set_merkle_root","set_signature","set_treasury","set_burnfee","set_difficulty","set_hash","generate_hash","generate_merkle_root","on_chain_reorganization","validate","blockchain","Blockchain","new","add_block","add_block_success","add_block_failure","get_latest_block","get_latest_block_hash","get_latest_block_id","is_new_chain_the_longest_chain","validate","wind_chain","unwind_chain","blockring","RingItem","new","contains_block_hash","add_block","on_chain_reorganization","BlockRing","new","print_lc","contains_block_hash_at_block_id","add_block","on_chain_reorganization","get_longest_chain_block_hash_by_block_id","get_longest_chain_block_hash","get_longest_chain_block_id","consensus","SaitoMessage","MempoolNewBlock","hash","MempoolNewTransaction","transaction","run","crypto","Message","PublicKey","SecretKey","Signature","SECP256K1","SaitoHash","SaitoUTXOSetKey","SaitoPublicKey","SaitoPrivateKey","SaitoSignature","PARALLEL_HASH_BYTE_THRESHOLD","generate_keys","hash","sign","verify","mempool","MempoolMessage","GenerateBlock","ProcessBlocks","AddBlockResult","Accepted","Exists","Mempool","new","add_block","generate_block","run","network","run","slip","SlipType","Normal","SlipCore","new","Slip","new","get_publickey","get_amount","get_uuid","get_slip_ordinal","get_slip_type","set_publickey","set_amount","set_uuid","set_slip_ordinal","set_slip_type","serialize_for_signature","on_chain_reorganization","get_utxoset_key","validate","time","create_timestamp","format_timestamp","TracingTimer","new","time_since_last","TracingAccumulator","new","set_start","accumulate_time_since_start","finish","transaction","TransactionType","Normal","TransactionCore","new","Transaction","new","add_input","add_output","get_timestamp","get_transaction_type","get_inputs","get_outputs","get_message","get_signature","set_timestamp","set_transaction_type","set_message","set_signature","set_hash_for_signature","sign","serialize_for_signature","on_chain_reorganization","validate","wallet","Wallet","new","get_privatekey","get_publickey","sign","Error","Result","from","into","to_owned","clone_into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","to_owned","clone_into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","to_owned","clone_into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","to_owned","clone_into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","equivalent","from","into","to_owned","clone_into","to_string","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","get_hash","vzip","equivalent","from","into","to_owned","clone_into","to_string","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","equivalent","from","into","to_owned","clone_into","to_string","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","equivalent","from","into","to_owned","clone_into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","to_owned","clone_into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","to_owned","clone_into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","get_hash","vzip","equivalent","from","into","to_owned","clone_into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","to_owned","clone_into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","to_owned","clone_into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","to_owned","clone_into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","to_owned","clone_into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","from","into","to_owned","clone_into","borrow","borrow_mut","try_from","try_into","type_id","init","deref","deref_mut","drop","vzip","deserialize","deserialize","deserialize","hash","cmp","cmp","cmp","fmt","fmt","fmt","fmt","fmt","fmt","fmt","serialize","serialize","serialize","as_ref","as_ref","as_c_ptr","as_mut_c_ptr","as_c_ptr","as_mut_c_ptr","as_c_ptr","as_mut_c_ptr","as_c_ptr","as_mut_c_ptr","from","from","from","eq","eq","ne","eq","ne","eq","index","index","index","index","index","index","index","index","index","index","from_str","from_str","from_str","fmt","fmt","clone","clone","clone","clone","partial_cmp","partial_cmp","partial_cmp","from","into","clone","clone","clone","clone","clone","clone","clone","clone","clone","clone","clone","clone","default","default","default","default","default","default","eq","ne","eq","ne","eq","eq","eq","ne","eq","ne","eq","eq","ne","eq","ne","fmt","fmt","fmt","fmt","fmt","fmt","fmt","fmt","fmt","fmt","fmt","fmt","fmt","fmt","hash","serialize","serialize","serialize","serialize","serialize","serialize","serialize","serialize","deserialize","deserialize","deserialize","deserialize","deserialize","deserialize","deserialize","deserialize","from_slice","as_ptr","as_mut_ptr","len","is_empty","as_ptr","as_mut_ptr","from_secret_key","from_slice","serialize","serialize_uncompressed","negate_assign","add_exp_assign","mul_assign","combine","combine_keys","new","from_slice","negate_assign","add_assign","mul_assign","as_ptr","as_mut_ptr","len","is_empty","from_der","from_compact","from_der_lax","normalize_s","as_ptr","as_mut_ptr","serialize_der","serialize_compact"],"q":["saito_rust","saito_rust::big_array","","","saito_rust","saito_rust::block","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","saito_rust","saito_rust::blockchain","","","","","","","","","","","","saito_rust","saito_rust::blockring","","","","","","","","","","","","","","saito_rust","saito_rust::consensus","","saito_rust::consensus::SaitoMessage","saito_rust::consensus","saito_rust::consensus::SaitoMessage","saito_rust::consensus","saito_rust","saito_rust::crypto","","","","","","","","","","","","","","","saito_rust","saito_rust::mempool","","","","","","","","","","","saito_rust","saito_rust::network","saito_rust","saito_rust::slip","","","","","","","","","","","","","","","","","","","","saito_rust","saito_rust::time","","","","","","","","","","saito_rust","saito_rust::transaction","","","","","","","","","","","","","","","","","","","","","","","saito_rust","saito_rust::wallet","","","","","saito_rust","","saito_rust::block","","","","","","","","","","","","","","","","","","","","","","","","","","","","saito_rust::blockchain","","","","","","","","","","","","saito_rust::blockring","","","","","","","","","","","","","","","","","","","","","","","","saito_rust::consensus","","","","","","","","","","","","","","saito_rust::crypto","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","saito_rust::mempool","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","saito_rust::slip","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","saito_rust::time","","","","","","","","","","","","","","","","","","","","","","","","saito_rust::transaction","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","saito_rust::wallet","","","","","","","","","","","","","","saito_rust::crypto","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","saito_rust::block","","","","saito_rust::consensus","saito_rust::mempool","","saito_rust::slip","","","saito_rust::transaction","","","saito_rust::wallet","saito_rust::block","","saito_rust::slip","","saito_rust::transaction","","saito_rust::block","","","","saito_rust::mempool","saito_rust::slip","","","","","saito_rust::transaction","","","","","saito_rust::block","","saito_rust::blockchain","saito_rust::blockring","","saito_rust::consensus","saito_rust::mempool","saito_rust::slip","","","saito_rust::transaction","","","saito_rust::wallet","saito_rust::slip","saito_rust::block","","saito_rust::slip","","","saito_rust::transaction","","","saito_rust::block","","saito_rust::slip","","","saito_rust::transaction","","","saito_rust::crypto","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","",""],"d":["","","","","","BlockCore is a self-contained object containing only the …","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","Create new <code>BlockRing</code>","","","","","","","","","The types of messages broadcast over the main broadcast …","","","","","Run the Saito consensus runtime","","A (hashed) message input to an ECDSA signature","A Secp256k1 public key, used for verification of …","Secret 256-bit key used as <code>x</code> in an ECDSA signature","An ECDSA signature","A global, static context to avoid repeatedly creating …","","","","","","","","","","","","","","","","","","The <code>Mempool</code> holds unprocessed blocks and transactions and …","","","","","","","","SlipType is a human-readable indicator of the slip-type, …","","SlipCore is a self-contained object containing only the …","","","","","","","","","","","","","","","","","","","","","A helper for tracing. Get the amount of time passed. Only …","Create a new TracingTimer","Gets the time passed since this method was called","","Create a new TracingAccumulator","","Accumulate the time passed since this method was called","Get the total time accumulated","","TransactionType is a human-readable indicator of the type …","","TransactionCore is a self-contained object containing …","","","","","","","","","","","","","","","","","","","","","","The <code>Wallet</code> manages the public and private keypair of the …","","","","","Error returned by most functions.","A specialized <code>Result</code> type for operations.","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","Gets a reference to the underlying array","Gets a reference to the underlying array","","","","","","","","","Converts a 32-byte hash directly to a message without …","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","<strong>If you just want to sign an arbitrary message use …","Converts the object to a raw pointer for FFI interfacing","Converts the object to a mutable raw pointer for FFI …","Returns the length of the object as an array","Returns whether the object as an array is empty","Obtains a raw const pointer suitable for use with FFI …","Obtains a raw mutable pointer suitable for use with FFI …","Creates a new public key from a secret key.","Creates a public key directly from a slice","Serialize the key as a byte-encoded pair of values. In …","Serialize the key as a byte-encoded pair of values, in …","Negates the pk to the pk <code>self</code> in place Will return an …","Adds the pk corresponding to <code>other</code> to the pk <code>self</code> in place…","Muliplies the pk <code>self</code> in place by the scalar <code>other</code> Will …","Adds a second key to this one, returning the sum. Returns …","Adds the keys in the provided slice together, returning …","Creates a new random secret key. Requires compilation …","Converts a <code>SECRET_KEY_SIZE</code>-byte slice to a secret key","Negates one secret key.","Adds one secret key to another, modulo the curve order. …","Multiplies one secret key by another, modulo the curve …","Converts the object to a raw pointer for FFI interfacing","Converts the object to a mutable raw pointer for FFI …","Returns the length of the object as an array","Returns whether the object as an array is empty","Converts a DER-encoded byte slice to a signature","Converts a 64-byte compact-encoded byte slice to a …","Converts a “lax DER”-encoded byte slice to a …","Normalizes a signature to a “low S” form. In ECDSA, …","Obtains a raw pointer suitable for use with FFI functions","Obtains a raw mutable pointer suitable for use with FFI …","Serializes the signature in DER format","Serializes the signature in compact format"],"i":[0,0,1,1,0,0,2,0,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,0,0,4,4,4,4,4,4,4,4,4,4,4,0,0,5,5,5,5,0,6,6,6,6,6,6,6,6,0,0,7,8,7,9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,10,10,0,11,11,0,12,12,12,0,0,0,0,0,13,0,14,0,15,15,15,15,15,15,15,15,15,15,15,15,15,15,15,0,0,0,0,16,16,0,17,17,17,17,0,0,18,0,19,0,20,20,20,20,20,20,20,20,20,20,20,20,20,20,20,20,20,20,0,0,21,21,21,21,0,0,2,2,2,2,2,2,2,2,2,2,2,2,2,2,3,3,3,3,3,3,3,3,3,3,3,3,3,3,4,4,4,4,4,4,4,4,4,4,4,4,5,5,5,5,5,5,5,5,5,5,5,5,6,6,6,6,6,6,6,6,6,6,6,6,7,7,7,7,7,7,7,7,7,7,7,7,7,7,22,22,22,22,22,22,22,22,22,22,22,22,22,22,22,23,23,23,23,23,23,23,23,23,23,23,23,23,23,23,23,23,24,24,24,24,24,24,24,24,24,24,24,24,24,24,24,24,25,25,25,25,25,25,25,25,25,25,25,25,25,25,25,25,10,10,10,10,10,10,10,10,10,10,10,10,10,10,11,11,11,11,11,11,11,11,11,11,11,11,11,11,12,12,12,12,12,12,12,12,12,12,12,12,13,13,13,13,13,13,13,13,13,13,13,13,13,13,13,13,14,14,14,14,14,14,14,14,14,14,14,14,14,14,15,15,15,15,15,15,15,15,15,15,15,15,15,15,16,16,16,16,16,16,16,16,16,16,16,16,17,17,17,17,17,17,17,17,17,17,17,17,18,18,18,18,18,18,18,18,18,18,18,18,18,18,19,19,19,19,19,19,19,19,19,19,19,19,19,19,20,20,20,20,20,20,20,20,20,20,20,20,20,20,21,21,21,21,21,21,21,21,21,21,21,21,21,21,24,23,25,23,24,23,22,23,24,25,25,23,22,24,23,25,24,24,22,24,24,23,23,25,25,22,22,22,25,23,22,23,23,25,25,24,22,22,24,24,22,22,24,24,22,24,24,23,25,24,23,25,24,22,23,24,22,23,3,3,2,3,7,10,11,13,14,15,18,19,20,21,2,3,14,15,19,20,2,2,3,3,11,13,14,14,15,15,18,19,19,20,20,2,3,4,5,6,7,10,13,14,15,18,19,20,21,13,2,3,13,14,15,18,19,20,2,3,13,14,15,18,19,20,22,22,22,22,22,23,23,23,23,23,23,23,23,23,23,23,24,24,24,24,24,24,24,24,24,25,25,25,25,25,25,25,25],"f":[null,null,[[],["result",4]],[[],["result",4]],null,null,[[["u64",15]]],null,[[["blockcore",3]],["block",3]],[[],["saitohash",6]],[[],["bool",15]],[[],["u64",15]],[[],["u64",15]],[[],["saitohash",6]],[[],["saitopublickey",6]],[[],["saitohash",6]],[[],["saitosignature",6]],[[],["u64",15]],[[],["u64",15]],[[],["u64",15]],[[["u64",15]]],[[["bool",15]]],[[["u64",15]]],[[["saitohash",6]]],[[["saitopublickey",6]]],[[["saitohash",6]]],[[]],[[["u64",15]]],[[["u64",15]]],[[["u64",15]]],[[["saitohash",6]]],[[],["saitohash",6]],[[],["saitohash",6]],[[["bool",15],["ahashmap",3]],["bool",15]],[[],["bool",15]],null,null,[[]],[[["block",3]]],[[["saitohash",6]]],[[]],[[],[["option",4],["block",3]]],[[],["saitohash",6]],[[],["u64",15]],[[["vec",3]],["bool",15]],[[["vec",3]],["bool",15]],[[["usize",15],["bool",15],["vec",3]],["bool",15]],[[["usize",15],["bool",15],["vec",3]],["bool",15]],null,null,[[]],[[["saitohash",6]],["bool",15]],[[["saitohash",6],["u64",15]]],[[["saitohash",6],["bool",15]]],null,[[]],[[]],[[["saitohash",6],["u64",15]],["bool",15]],[[["block",3]]],[[["saitohash",6],["bool",15],["u64",15]]],[[["u64",15]],["saitohash",6]],[[],["saitohash",6]],[[],["u64",15]],null,null,null,null,null,null,[[]],null,null,null,null,null,null,null,null,null,null,null,null,[[]],[[["vec",3]],["saitohash",6]],[[["saitoprivatekey",6]],["saitosignature",6]],[[["saitopublickey",6],["saitosignature",6]],["bool",15]],null,null,null,null,null,null,null,null,[[["rwlock",3],["arc",3]]],[[["block",3]],["addblockresult",4]],[[["arc",3],["rwlock",3]]],[[["sender",3],["arc",3],["arc",3],["saitomessage",4],["rwlock",3],["receiver",3],["rwlock",3]]],null,[[["sender",3],["receiver",3],["saitomessage",4]]],null,null,null,null,[[["u8",15],["sliptype",4],["u64",15]]],null,[[["slipcore",3]]],[[],["saitopublickey",6]],[[],["u64",15]],[[],["saitosignature",6]],[[],["u8",15]],[[],["sliptype",4]],[[["saitopublickey",6]]],[[["u64",15]]],[[["saitosignature",6]]],[[["u8",15]]],[[["sliptype",4]]],[[],[["vec",3],["u8",15]]],[[["bool",15],["ahashmap",3],["u64",15]]],[[],["saitoutxosetkey",6]],[[],["bool",15]],null,[[],["u64",15]],[[["u64",15]],[["delayedformat",3],["strftimeitems",3]]],null,[[],["tracingtimer",3]],[[],["u64",15]],null,[[],["tracingaccumulator",3]],[[]],[[]],[[],["u64",15]],null,null,null,null,[[["saitosignature",6],["vec",3],["u64",15],["slip",3],["vec",3],["u8",15],["transactiontype",4]]],null,[[["transactioncore",3]]],[[["slip",3]]],[[["slip",3]]],[[],["u64",15]],[[],["transactiontype",4]],[[],["vec",3]],[[],["vec",3]],[[],["vec",3]],[[]],[[["u64",15]]],[[["transactiontype",4]]],[[["vec",3],["u8",15]]],[[["saitosignature",6]]],[[["saitohash",6]]],[[["saitoprivatekey",6]]],[[],[["vec",3],["u8",15]]],[[["bool",15],["ahashmap",3],["u64",15]]],[[],["bool",15]],null,null,[[]],[[],["saitoprivatekey",6]],[[],["saitopublickey",6]],[[],["saitosignature",6]],null,null,[[]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[],["bool",15]],[[]],[[]],[[]],[[]],[[],["string",3]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[],["u64",15]],[[]],[[],["bool",15]],[[]],[[]],[[]],[[]],[[],["string",3]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[],["bool",15]],[[]],[[]],[[]],[[]],[[],["string",3]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[],["bool",15]],[[]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[],["u64",15]],[[]],[[],["bool",15]],[[]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["usize",15]],[[["usize",15]]],[[["usize",15]]],[[["usize",15]]],[[]],[[],[["result",4],["secretkey",3]]],[[],[["publickey",3],["result",4]]],[[],[["result",4],["signature",3]]],[[]],[[["secretkey",3]],["ordering",4]],[[["publickey",3]],["ordering",4]],[[["message",3]],["ordering",4]],[[["formatter",3]],[["result",4],["error",3]]],[[["formatter",3]],[["result",4],["error",3]]],[[["formatter",3]],[["result",4],["error",3]]],[[["formatter",3]],[["result",4],["error",3]]],[[["formatter",3]],[["result",4],["error",3]]],[[["formatter",3]],[["result",4],["error",3]]],[[["formatter",3]],[["result",4],["error",3]]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[],["message",3]],[[["signature",3]],["signature",3]],[[["publickey",3]],["publickey",3]],[[["message",3]],["bool",15]],[[["publickey",3]],["bool",15]],[[["publickey",3]],["bool",15]],[[["signature",3]],["bool",15]],[[["signature",3]],["bool",15]],[[["secretkey",3]],["bool",15]],[[["usize",15],["rangeto",3]]],[[["usize",15]],["u8",15]],[[["usize",15]],["u8",15]],[[["range",3],["usize",15]]],[[["range",3],["usize",15]]],[[["rangefull",3]]],[[["usize",15],["rangeto",3]]],[[["usize",15],["rangefrom",3]]],[[["usize",15],["rangefrom",3]]],[[["rangefull",3]]],[[["str",15]],[["secretkey",3],["error",4],["result",4]]],[[["str",15]],[["result",4],["publickey",3],["error",4]]],[[["str",15]],[["error",4],["result",4],["signature",3]]],[[["formatter",3]],[["result",4],["error",3]]],[[["formatter",3]],[["result",4],["error",3]]],[[],["signature",3]],[[],["secretkey",3]],[[],["message",3]],[[],["publickey",3]],[[["secretkey",3]],[["ordering",4],["option",4]]],[[["message",3]],[["ordering",4],["option",4]]],[[["publickey",3]],[["ordering",4],["option",4]]],[[["vec",3],["u8",15]]],[[],[["vec",3],["u8",15]]],[[],["blockcore",3]],[[],["block",3]],[[],["saitomessage",4]],[[],["mempoolmessage",4]],[[],["addblockresult",4]],[[],["sliptype",4]],[[],["slipcore",3]],[[],["slip",3]],[[],["transactiontype",4]],[[],["transactioncore",3]],[[],["transaction",3]],[[],["wallet",3]],[[]],[[]],[[]],[[]],[[]],[[]],[[["blockcore",3]],["bool",15]],[[["blockcore",3]],["bool",15]],[[["block",3]],["bool",15]],[[["block",3]],["bool",15]],[[["addblockresult",4]],["bool",15]],[[["sliptype",4]],["bool",15]],[[["slipcore",3]],["bool",15]],[[["slipcore",3]],["bool",15]],[[["slip",3]],["bool",15]],[[["slip",3]],["bool",15]],[[["transactiontype",4]],["bool",15]],[[["transactioncore",3]],["bool",15]],[[["transactioncore",3]],["bool",15]],[[["transaction",3]],["bool",15]],[[["transaction",3]],["bool",15]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],[["message",3],["result",4],["error",4]]],[[]],[[]],[[],["usize",15]],[[],["bool",15]],[[]],[[]],[[["secp256k1",3],["secretkey",3]],["publickey",3]],[[],[["result",4],["publickey",3],["error",4]]],[[]],[[]],[[["secp256k1",3]]],[[["secp256k1",3]],[["error",4],["result",4]]],[[["secp256k1",3]],[["error",4],["result",4]]],[[["publickey",3]],[["result",4],["publickey",3],["error",4]]],[[],[["result",4],["publickey",3],["error",4]]],[[],["secretkey",3]],[[],[["secretkey",3],["error",4],["result",4]]],[[]],[[],[["error",4],["result",4]]],[[],[["error",4],["result",4]]],[[]],[[]],[[],["usize",15]],[[],["bool",15]],[[],[["error",4],["result",4],["signature",3]]],[[],[["error",4],["result",4],["signature",3]]],[[],[["error",4],["result",4],["signature",3]]],[[]],[[]],[[]],[[],["serializedsignature",3]],[[]]],"p":[[8,"BigArray"],[3,"BlockCore"],[3,"Block"],[3,"Blockchain"],[3,"RingItem"],[3,"BlockRing"],[4,"SaitoMessage"],[13,"MempoolNewBlock"],[13,"MempoolNewTransaction"],[4,"MempoolMessage"],[4,"AddBlockResult"],[3,"Mempool"],[4,"SlipType"],[3,"SlipCore"],[3,"Slip"],[3,"TracingTimer"],[3,"TracingAccumulator"],[4,"TransactionType"],[3,"TransactionCore"],[3,"Transaction"],[3,"Wallet"],[3,"Message"],[3,"PublicKey"],[3,"SecretKey"],[3,"Signature"]]},\
"spammer":{"doc":"","t":[5],"n":["main"],"q":["spammer"],"d":[""],"i":[0],"f":[[[],["result",6]]],"p":[]}\
}');
initSearch(searchIndex);