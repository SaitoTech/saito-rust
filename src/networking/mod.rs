/*!

# Networking Interfaces and Methods

## Introduction

Saito provides a minimalistic binary RPC interface via websockets for node-to-node interaction.

Blockchain data is encoded as raw-binary for effeciency.

This enables high throughput and also 2-way(full-duplex) communication between nodes(which is important for things like new block notifications).

## Saito RPC

A basic SaitoRPC request looks like the following:

```
bytes
0-7     MessageName
8-11    MessageID(big-endian u32)(optional)
12..    MessageData
```

The MessageName is typically the method which the sender wishes to invoke, e.g. GETBLOCK. The ID is used to match requests to responses, similar to the ID in JSON-RPC.

MessageData is arbirary data whose form depends on the type of message(MessageName).

## MessageName

We reserve 8 bytes for RPC names so that we can easily add and deprecate methods if needed in the future.

Two special names are used for responses.

```
ERROR___
RESULT__
```

The basic methods are: Get Block, Get Block Header, Get Lite Block, New Block Announce, Send Transaction, Subscribe to Lite Blocks by pubkey or "module".
 
```
SHAKINIT
SHAKCOMP
GETBLKHD
NEWBLOCK
SENDTRXN
SUBBYKEY
SUBBYMOD
```

TODO: formalize and rename the concept of "module". Are there any fields beside pubkey and module which we'd like to use to whitelist tx in a lite block?

## Handshake

A node will not accept new socket connections unless it has completed a handshake with the client attempting to open the connection.

A handshake can be initialized by sending a GET request to /handshakeinit and then POSTing a signed challenge to /handshakecomplete(details below).

Serialized Handshake:

```
    challenger_ip_address   4 bytes(IP as 4 bytes)
    challengie_ip_address   4 bytes(IP as 4 bytes)
    challenger_pubkey       33 bytes(SECP256k1 compact form)
    challengie_pubkey       33 bytes(SECP256k1 compact form)
    timestamp               8 bytes(big-endian u64),
    challenger_sig          64 bytes(SECP256k1 signature)
    challengie_sig          64 bytes(SECP256k1 signature)(optional)
```

### 1) Initiate a handshake

Returns a serialized handshake signed by the challenger(the node which receives the request)

```
curl {endpoint}/handshakeinit\?a={pubkey} > test_challenge
```

### 2) Complete a handshake

The challengie must sign the handshakeinit payload. I.E. sign the entire blob and append the sig(challengie_sig) to it.

The wallet CLI can be used to produce a signed challenge.

```
cargo run --bin walletcli sign test_challenge signed_challenge
curl --data-binary "@signed_challenge" -X POST http://127.0.0.1:3030/handshakecomplete
```

If the payload is valid, this will return a 32 byte token.

### 3) Open a socket

The token can now be used to open a socket connection:

```
wscat -H socket-token:{token} -c ws://{endpoint}
```

## RPC Messages

### RESULT__

A response to an RPC method. MessageID is the id of the request it is responding to. Message encoding depends on the request.

MessageData:
```
bytes
0-7         MessageName(same as the request, for convenience so senders don't necessarily need to track MessageID to decode a response)
7..         Response Data
```

### ERROR___

A response to an RPC method.
MessageID is the id of the request it is responding to.

MessageData:
```
bytes
0..         UTF8 Encoded Error Message
```

### GETBLKHD

Get a block header(metadata) by block hash.

MessageData:
```
0-31        Block Hash
```

### NEWBLOCK

Informs a peer that the longest chain's tip has been grown with the given block id/hash.

MessageData:
```
0-7         Block ID(u64)
8-39        Block Hash
```

### SENDTRXN

Informs a peer that the longest chain's tip has been grown with the given block id/hash.

MessageData:
```
0-7         Block ID(u64)
8-39        Block Hash
```

### SUBBYKEY

Subscribe to be sent any new blocks as lite blocks

MessageData:
```
[32 bytes]  List of Pubkeys
```

### SUBBYMOD


Subscribe to be sent any new blocks as lite blocks

MessageData:
```
0..         Module Name
```

TODO: Add docs for SHAKINIT and SHAKCOMP

## RPC basic example 

After creating a token via /handshakeinit and /handshakecomplete:

```
websocat ws://127.0.0.1:3030/wsconnect -H socket-token:$TOKEN -b readfile:rpc_message_file.bin
```

## RPC complete example 

Run a node:

```
cargo run
```

In another shell, get your public key from the wallet CLI:

```
cargo run --bin walletcli print
```

Initialize a handshake using your pubkey:
```
curl 127.0.0.1:3030/handshakeinit\?a={pubkey} > test_challenge
```

E.G.:
```
curl 127.0.0.1:3030/handshakeinit\?a=02579d6ff84f661297f38e3eb20953824cfc279fee903a746b3dccb534677fd81a > test_challenge
```
Sign the payload with the wallet CLI:
```
cargo run --bin walletcli sign test_challenge signed_challenge
```

Send the signed challenge to /handshakecomplete:
```
curl --data-binary "@signed_challenge" -X POST http://127.0.0.1:3030/handshakecomplete > ws_token
```

Send some data to the socket
```
websocat ws://127.0.0.1:3030/wsconnect -H socket-token:$TOKEN -b readfile:some_binary_rpc_call.bin
```

There is also a script in scripts/socket.sh which can be used.

*/

pub mod handlers;
pub mod network;
pub mod filters;
pub mod socket;
