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

```
SHAKINIT
SHAKCOMP
REQCHAIN
SNDCHAIN
REQBLKHD
SNDBLKHD
SNDTRANS
REQBLOCK
SNDKYCHN
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

### SHAKINIT

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

### SHAKCOMP

The challengie must sign the handshakeinit payload. I.E. sign the entire blob and append the sig(challengie_sig) to it.

The wallet CLI can be used to produce a signed challenge.

### REQCHAIN

triggers: remote server should calculate appropriate response and "send blockchain"

MessageData:

```
0-7         Latest Block ID(u64)
8-39        Latest Block Hash
40-61       Fork ID
```

RESULT__ contains no data, indicates that the request was successful

### SNDCHAIN

TODO

### SNDBLKHD

TODO

### REQBLKHD

TODO

### SNDTRANS

TODO

### REQBLOCK

TODO



### SNDKYCHN

TODO

*/

pub mod filters;
pub mod handlers;
pub mod network;
pub mod socket;
