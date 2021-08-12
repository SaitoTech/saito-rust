use crate::crypto::{sign_blob, SaitoPrivateKey, SaitoPublicKey, SaitoSignature};

use crate::networking::api_message::APIMessage;
use crate::networking::network::CHALLENGE_SIZE;
use crate::time::create_timestamp;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct HandshakeChallenge {
    challenger_ip_address: [u8; 4],
    challengie_ip_address: [u8; 4],
    #[serde_as(as = "[_; 33]")]
    challenger_pubkey: SaitoPublicKey,
    #[serde_as(as = "[_; 33]")]
    challengie_pubkey: SaitoPublicKey,
    timestamp: u64,
}
impl HandshakeChallenge {
    pub fn new(
        challenger_ip_address: [u8; 4],
        challengie_ip_address: [u8; 4],
        challenger_pubkey: SaitoPublicKey,
        challengie_pubkey: SaitoPublicKey,
    ) -> HandshakeChallenge {
        HandshakeChallenge {
            challenger_ip_address: challenger_ip_address,
            challengie_ip_address: challengie_ip_address,
            challenger_pubkey: challenger_pubkey,
            challengie_pubkey: challengie_pubkey,
            timestamp: create_timestamp(),
        }
    }
    pub fn deserialize_raw(bytes: &Vec<u8>) -> HandshakeChallenge {
        let mut challenger_octet: [u8; 4] = [0; 4];
        challenger_octet[0..4].clone_from_slice(&bytes[0..4]);
        let mut challengie_octet: [u8; 4] = [0; 4];
        challengie_octet[0..4].clone_from_slice(&bytes[4..8]);

        let challenger_pubkey: SaitoPublicKey = bytes[8..41].try_into().unwrap();
        let challengie_pubkey: SaitoPublicKey = bytes[41..74].try_into().unwrap();
        let timestamp: u64 = u64::from_be_bytes(bytes[74..CHALLENGE_SIZE].try_into().unwrap());

        HandshakeChallenge {
            challenger_ip_address: challenger_octet,
            challengie_ip_address: challengie_octet,
            challenger_pubkey: challenger_pubkey,
            challengie_pubkey: challengie_pubkey,
            timestamp: timestamp,
        }
    }
    pub fn deserialize_with_sig(bytes: &Vec<u8>) -> (HandshakeChallenge, SaitoSignature) {
        let handshake_challenge = HandshakeChallenge::deserialize_raw(bytes);
        let signature: SaitoSignature = bytes[CHALLENGE_SIZE..CHALLENGE_SIZE + 64]
            .try_into()
            .unwrap();
        (handshake_challenge, signature)
    }
    pub fn deserialize_with_both_sigs(
        bytes: &Vec<u8>,
    ) -> (HandshakeChallenge, SaitoSignature, SaitoSignature) {
        let handshake_challenge = HandshakeChallenge::deserialize_raw(bytes);
        let signature1: SaitoSignature = bytes[CHALLENGE_SIZE..CHALLENGE_SIZE + 64]
            .try_into()
            .unwrap();
        let signature2: SaitoSignature = bytes[CHALLENGE_SIZE + 64..CHALLENGE_SIZE + 128]
            .try_into()
            .unwrap();
        (handshake_challenge, signature1, signature2)
    }
    pub fn serialize_raw(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.challenger_ip_address);
        vbytes.extend(&self.challengie_ip_address);
        vbytes.extend(&self.challenger_pubkey);
        vbytes.extend(&self.challengie_pubkey);
        vbytes.extend(&self.timestamp.to_be_bytes());
        vbytes
    }
    pub fn serialize_with_sig(&self, privatekey: SaitoPrivateKey) -> Vec<u8> {
        sign_blob(&mut self.serialize_raw(), privatekey).to_owned()
    }

    pub fn challenger_ip_address(&self) -> [u8; 4] {
        self.challenger_ip_address
    }
    pub fn challengie_ip_address(&self) -> [u8; 4] {
        self.challengie_ip_address
    }
    pub fn challenger_pubkey(&self) -> SaitoPublicKey {
        self.challenger_pubkey
    }
    pub fn challengie_pubkey(&self) -> SaitoPublicKey {
        self.challengie_pubkey
    }
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        crypto::generate_keys, networking::message_types::handshake_challenge::HandshakeChallenge,
        time::create_timestamp,
    };

    #[tokio::test]
    async fn test_challenge_serialize() {
        let (publickey, privatekey) = generate_keys();
        let challenge = HandshakeChallenge {
            challenger_ip_address: [127, 0, 0, 1],
            challengie_ip_address: [127, 0, 0, 1],
            challenger_pubkey: publickey,
            challengie_pubkey: publickey,
            timestamp: create_timestamp(),
        };
        let serialized_challenge = challenge.serialize_with_sig(privatekey);
        let deserialized_challenge =
            HandshakeChallenge::deserialize_with_sig(&serialized_challenge);
        assert_eq!(challenge, deserialized_challenge.0);
    }
}
