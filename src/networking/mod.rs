/*!

# Networking Interfaces and Methods

## Introduction

Saito provides a minimalistic binary RPC interface via websockets for node-to-node interaction.

Blockchain data is encoded as raw-binary.

This enables high throughput and also 2-way(full-duplex) communication between nodes(which is important for things like new block notifications).

## Saito RPC

A basic SaitoRPC request looks like the following:

```bytes
0-7     MessageName
8-11    MessageID(big-endian u32)(optional)
12..    MessageData
```

The `APIMessage` type carries this data.

MessageName is typically the method which the sender wishes to invoke, e.g. GETBLOCK. The ID is used to match requests to responses, similar to the ID in JSON-RPC.

MessageData is arbitrary data whose form depends on the type of message(MessageName).

MessageData should be encoded in a type in networking/message_types for clarity and type-safety.

## MessageName

We reserve 8 bytes for RPC names so that we can easily add and deprecate methods if needed in the future.

Two special names are used for responses.

```bytes
ERROR___
RESULT__
```

```bytes
SHAKINIT
SHAKCOMP
REQCHAIN
SNDCHAIN
REQBLKHD
SNDBLKHD
SNDTRANS
REQBLOCK
SNDKYLST
```

## RPC Messages

### RESULT__

A response to an RPC method. MessageID is the id of the request it is responding to. Message encoding depends on the request.

MessageData:
```bytes
0-7         MessageName(same as the request, for convenience so senders don't necessarily need to track MessageID to decode a response)
7..         Response Data
```

### ERROR___

An error response to an RPC method.
MessageID is the id of the request it is responding to.

TODO: Add a basic type to networking/message_types that just wraps a string for error messages.

MessageData:
```bytes
0..         UTF8 Encoded Error Message
```

### SHAKINIT

After opening a socket a node should initialize a handshake via SHAKINIT with a `HandshakeChallenge` payload.

### SHAKCOMP

The opponent must sign the SHAKINIT payload. I.E. sign the entire blob and append the sig(opponent_sig) to it.

The Saito CLI can be used to produce a signed challenge.

### REQCHAIN

DETAILS OF THESE COMMAND ARE CURRENTLY IN A WORKING DOCUMENT WHICH CAN BE SHARED UPON REQUEST.

TODO

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



### SNDKYLST

TODO

*/

pub mod api_message;
pub mod filters;
pub mod handlers;
pub mod message_types;
pub mod network;
pub mod peer;
