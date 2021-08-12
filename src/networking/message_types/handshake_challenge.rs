use crate::crypto::{sign_blob, SaitoPrivateKey, SaitoPublicKey, SaitoSignature};

use crate::networking::network::CHALLENGE_SIZE;
use crate::time::create_timestamp;
use std::convert::TryInto;

#[derive(Debug, PartialEq)]
pub struct HandshakeChallenge {
    pub challenger_node: HandshakeNode,
    pub opponent_node: HandshakeNode,
    pub timestamp: u64
}

#[derive(Debug, PartialEq)]
pub struct HandshakeNode{
    pub ip_address: [u8; 4],
    pub public_key: SaitoPublicKey,
    pub sig: Option<SaitoSignature>,
}

impl HandshakeChallenge {
    pub fn new(
        (challenger_ip_address, challenger_public_key): ([u8; 4], SaitoPublicKey),
        (opponent_ip_adress, opponent_public_key): ([u8; 4], SaitoPublicKey),
    ) -> Self {
        Self {
            challenger_node: HandshakeNode {
                ip_address: challenger_ip_address,
                public_key: challenger_public_key,
                sig: None
            },
            opponent_node: HandshakeNode {
                ip_address: opponent_ip_adress,
                public_key: opponent_public_key,
                sig: None
            },
            timestamp: create_timestamp()
        }
    }

    pub fn deserialize(bytes: &Vec<u8>) -> HandshakeChallenge {
        let mut challenger_octet: [u8; 4] = [0; 4];
        challenger_octet[0..4].clone_from_slice(&bytes[0..4]);
        let mut opponent_octet: [u8; 4] = [0; 4];
        opponent_octet[0..4].clone_from_slice(&bytes[4..8]);

        let challenger_pubkey: SaitoPublicKey = bytes[8..41].try_into().unwrap();
        let opponent_pubkey: SaitoPublicKey = bytes[41..74].try_into().unwrap();
        let timestamp: u64 = u64::from_be_bytes(bytes[74..CHALLENGE_SIZE].try_into().unwrap());

        let mut handshake_challenge = HandshakeChallenge::new(
            (challenger_octet, challenger_pubkey),
            (opponent_octet, opponent_pubkey),
        );

        handshake_challenge.set_timestamp(timestamp);

        if bytes.len() > CHALLENGE_SIZE {
            handshake_challenge.set_challenger_sig(Some(bytes[CHALLENGE_SIZE..CHALLENGE_SIZE + 64].try_into().unwrap()));
        }

        if bytes.len() > CHALLENGE_SIZE + 64 {
            handshake_challenge.set_opponent_sig(Some(bytes[CHALLENGE_SIZE + 64..CHALLENGE_SIZE + 128].try_into().unwrap()));
        }
        handshake_challenge
    }

    pub fn serialize_raw(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.challenger_node.ip_address);
        vbytes.extend(&self.opponent_node.ip_address);
        vbytes.extend(&self.challenger_node.public_key);
        vbytes.extend(&self.opponent_node.public_key);
        vbytes.extend(&self.timestamp.to_be_bytes());
        vbytes
    }

    pub fn serialize_with_sig(&self, privatekey: SaitoPrivateKey) -> Vec<u8> {
        sign_blob(&mut self.serialize_raw(), privatekey).to_owned()
    }

    pub fn challenger_ip_address(&self) -> [u8; 4] {
        self.challenger_node.ip_address
    }

    pub fn opponent_ip_address(&self) -> [u8; 4] {
        self.opponent_node.ip_address
    }

    pub fn challenger_pubkey(&self) -> SaitoPublicKey {
        self.challenger_node.public_key
    }

    pub fn opponent_pubkey(&self) -> SaitoPublicKey {
        self.opponent_node.public_key
    }

    pub fn challenger_sig(&self) -> Option<SaitoSignature> {
        self.challenger_node.sig
    }

    pub fn opponent_sig(&self) -> Option<SaitoSignature> {
        self.opponent_node.sig
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.timestamp = timestamp;
    }

    pub fn set_challenger_sig(&mut self, sig: Option<SaitoSignature>) {
        self.challenger_node.sig = sig
    }

    pub fn set_opponent_sig(&mut self, sig: Option<SaitoSignature>) {
        self.opponent_node.sig = sig
    }
}


#[cfg(test)]
mod tests {
    use crate::{
        crypto::generate_keys, networking::message_types::handshake_challenge::HandshakeChallenge,
    };

    #[tokio::test]
    async fn test_challenge_serialize() {
        let (publickey, privatekey) = generate_keys();
        let challenge = HandshakeChallenge::new(
            ([127, 0, 0, 1], publickey),
            ([127, 0, 0, 1], publickey)
        );

        let serialized_challenge = challenge.serialize_with_sig(privatekey);
        let deserialized_challenge = HandshakeChallenge::deserialize(&serialized_challenge);

        assert_eq!(challenge.challenger_ip_address(), deserialized_challenge.challenger_ip_address());
        assert_eq!(challenge.challenger_ip_address(), deserialized_challenge.challenger_ip_address());

        assert_eq!(challenge.challenger_pubkey(), deserialized_challenge.challenger_pubkey());
        assert_eq!(challenge.opponent_pubkey(), deserialized_challenge.opponent_pubkey());

        assert_eq!(challenge.timestamp, deserialized_challenge.timestamp);
    }
}
