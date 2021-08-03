use crate::blockchain::Blockchain;
use crate::consensus::SaitoMessage;
use crate::crypto::{sign_blob, SaitoHash, SaitoPrivateKey, SaitoPublicKey, SaitoSignature};
use crate::mempool::Mempool;
use crate::networking::filters::{
    get_block_route_filter, post_block_route_filter, post_transaction_route_filter,
    ws_upgrade_route_filter,
};
use crate::time::create_timestamp;
use crate::wallet::Wallet;
use tokio::sync::{broadcast, mpsc, RwLock};
use warp::ws::Message;

use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::Arc;
use warp::{Filter, Rejection};

use serde::{Deserialize, Serialize};

pub const CHALLENGE_SIZE: usize = 82;
pub const CHALLENGE_EXPIRATION_TIME: u64 = 60000;

pub type Result<T> = std::result::Result<T, Rejection>;
pub type Peers = Arc<RwLock<HashMap<SaitoHash, Peer>>>;

#[derive(Debug, Clone)]
pub struct Peer {
    pub has_handshake: bool,
    pub pubkey: Option<SaitoPublicKey>,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct APIMessage {
    pub message_name: [u8; 8],
    pub message_id: u32,
    pub message_data: Vec<u8>,
}

impl APIMessage {
    pub fn new(message_name: &str, message_id: u32, message_data: Vec<u8>) -> APIMessage {
        APIMessage {
            message_name: String::from(message_name).as_bytes().try_into().unwrap(),
            message_id: message_id,
            message_data: message_data,
        }
    }
    pub fn message_name(&self) -> &[u8; 8] {
        &self.message_name
    }
    pub fn message_id(&self) -> u32 {
        self.message_id
    }
    pub fn message_data(&self) -> &Vec<u8> {
        &self.message_data
    }
    pub fn new_with_data_from_str(
        message_name: &str,
        message_id: u32,
        message_data: &str,
    ) -> APIMessage {
        APIMessage {
            message_name: String::from(message_name).as_bytes().try_into().unwrap(),
            message_id: message_id,
            message_data: String::from(message_data).as_bytes().try_into().unwrap(),
        }
    }
    pub fn deserialize(bytes: &Vec<u8>) -> APIMessage {
        let message_name: [u8; 8] = bytes[0..8].try_into().unwrap();
        let message_id: u32 = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
        let message_data = bytes[12..].try_into().unwrap();
        APIMessage {
            message_name: message_name,
            message_id: message_id,
            message_data: message_data,
        }
    }
    pub fn serialize(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.message_name);
        vbytes.extend(&self.message_id.to_be_bytes());
        vbytes.extend(&self.message_data);
        vbytes
    }
}
pub struct Network {
    peers: Peers,
    wallet_lock: Arc<RwLock<Wallet>>,
}

impl Network {
    pub fn new(wallet_lock: Arc<RwLock<Wallet>>) -> Network {
        Network {
            peers: Arc::new(RwLock::new(HashMap::new())),
            wallet_lock,
        }
    }

    pub async fn run(
        &self,
        mempool_lock: Arc<RwLock<Mempool>>,
        blockchain_lock: Arc<RwLock<Blockchain>>,
        _broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
        _broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
    ) -> crate::Result<()> {
        println!("network run");
        // while let Ok(_message) = broadcast_channel_receiver.recv().await {
        //     //println!("NEW BLOCK!");
        // }

        let routes = get_block_route_filter()
            .or(post_transaction_route_filter())
            .or(post_block_route_filter())
            .or(ws_upgrade_route_filter(
                &self.peers.clone(),
                self.wallet_lock.clone(),
                mempool_lock.clone(),
                blockchain_lock.clone(),
            ));
        warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
        Ok(())
    }
}

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
    use super::*;
    use crate::{
        crypto::{generate_keys, hash, verify, SaitoSignature},
        mempool::Mempool,
        networking::filters::ws_upgrade_route_filter,
        transaction::Transaction,
    };
    use secp256k1::PublicKey;

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
    #[tokio::test]
    async fn test_message_serialize() {
        let api_message = APIMessage {
            //message_name: ["1", "2", "3"], //String::from("HLLOWRLD").as_bytes().clone(),
            message_name: String::from("HLLOWRLD").as_bytes().try_into().unwrap(),
            message_id: 1,
            message_data: String::from("SOMEDATA").as_bytes().try_into().unwrap(),
        };
        let serialized_api_message = api_message.serialize();
        let deserialized_api_message = APIMessage::deserialize(&serialized_api_message);
        assert_eq!(api_message, deserialized_api_message);
    }
    #[tokio::test]
    async fn test_handshake_new() {
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let network = Network::new(wallet_lock.clone());
        let (publickey, privatekey) = generate_keys();

        let socket_filter = ws_upgrade_route_filter(
            &network.peers.clone(),
            wallet_lock.clone(),
            mempool_lock.clone(),
            blockchain_lock.clone(),
        );
        let mut ws_client = warp::test::ws()
            .path("/wsopen")
            .handshake(socket_filter)
            .await
            .expect("handshake");

        // let base58_pubkey = PublicKey::from_slice(&publickey).unwrap().serialize().to_base58();
        let mut message_data = vec![127, 0, 0, 1];
        message_data.extend(
            PublicKey::from_slice(&publickey)
                .unwrap()
                .serialize()
                .to_vec(),
        );
        let api_message = APIMessage::new("SHAKINIT", 42, message_data);

        let serialized_api_message = api_message.serialize();

        let _socket_resp = ws_client
            .send(Message::binary(serialized_api_message))
            .await;
        let resp = ws_client.recv().await.unwrap();

        let command = String::from_utf8_lossy(&resp.as_bytes()[0..8]);
        let index: u32 = u32::from_be_bytes(resp.as_bytes()[8..12].try_into().unwrap());

        assert_eq!(command, "RESULT__");
        assert_eq!(index, 42);

        let deserialize_challenge =
            HandshakeChallenge::deserialize_with_sig(&resp.as_bytes()[12..].to_vec());
        let raw_challenge: [u8; CHALLENGE_SIZE] =
            resp.as_bytes()[12..][..CHALLENGE_SIZE].try_into().unwrap();
        let sig: SaitoSignature = resp.as_bytes()[12..][CHALLENGE_SIZE..CHALLENGE_SIZE + 64]
            .try_into()
            .unwrap();

        assert_eq!(
            deserialize_challenge.0.challenger_ip_address(),
            [42, 42, 42, 42]
        );
        assert_eq!(
            deserialize_challenge.0.challengie_ip_address(),
            [127, 0, 0, 1]
        );
        assert_eq!(deserialize_challenge.0.challengie_pubkey(), publickey);
        assert!(verify(
            &hash(&raw_challenge.to_vec()),
            sig,
            deserialize_challenge.0.challenger_pubkey()
        ));

        let signed_challenge =
            sign_blob(&mut resp.as_bytes()[12..].to_vec(), privatekey).to_owned();

        let api_message = APIMessage::new("SHAKCOMP", 43, signed_challenge);
        let serialized_api_message = api_message.serialize();

        let _socket_resp = ws_client
            .send(Message::binary(serialized_api_message))
            .await;
        let resp = ws_client.recv().await.unwrap();

        let command = String::from_utf8_lossy(&resp.as_bytes()[0..8]);
        let index: u32 = u32::from_be_bytes(resp.as_bytes()[8..12].try_into().unwrap());
        let msg = String::from_utf8_lossy(&resp.as_bytes()[12..]);
        assert_eq!(command, "RESULT__");
        assert_eq!(index, 43);
        assert_eq!(msg, "OK");
    }

    #[tokio::test]
    async fn test_send_transaction() {
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let network = Network::new(wallet_lock.clone());
        let (publickey, _privatekey) = generate_keys();

        let socket_filter = ws_upgrade_route_filter(
            &network.peers.clone(),
            wallet_lock.clone(),
            mempool_lock.clone(),
            blockchain_lock.clone(),
        );
        let mut ws_client = warp::test::ws()
            .path("/wsopen")
            .handshake(socket_filter)
            .await
            .expect("transaction websocket");

        let mut message_data = vec![127, 0, 0, 1];
        message_data.extend(
            PublicKey::from_slice(&publickey)
                .unwrap()
                .serialize()
                .to_vec(),
        );
        let transaction = Transaction::new();
        let api_message = APIMessage::new("SENDTRXN", 0, transaction.serialize_for_net());
        let serialized_api_message = api_message.serialize();

        let _socket_resp = ws_client
            .send(Message::binary(serialized_api_message))
            .await;
        let _resp = ws_client.recv().await.unwrap();

        let mempool = mempool_lock.read().await;
        assert_eq!(mempool.transactions.len(), 1);
    }
}
