use crate::{crypto::SaitoSignature, slip::Slip, time::create_timestamp};
use serde::{Deserialize, Serialize};

/// TransactionType is a human-readable indicator of the type of
/// transaction such as a normal user-initiated transaction, a
/// golden ticket transaction, a VIP-transaction or a rebroadcast
/// transaction created by a block creator, etc.
#[derive(Serialize, Deserialize, Debug, Copy, PartialEq, Clone)]
pub enum TransactionType {
    Normal,
}

/// TransactionCore is a self-contained object containing only the core
/// information about the transaction that exists regardless of how it
/// was routed or produced. It exists to simplify transaction serialization
/// and deserialization until we have custom functions.
///
/// This is a private variable. Access to variables within the
/// TransactionCore should be handled through getters and setters in the
/// block which surrounds it.
#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TransactionCore {
    timestamp: u64,
    inputs: Vec<Slip>,
    outputs: Vec<Slip>,
    #[serde(with = "serde_bytes")]
    message: Vec<u8>,
    transaction_type: TransactionType,
    #[serde_as(as = "[_; 64]")]
    signature: SaitoSignature,
}

impl TransactionCore {
    pub fn new(
        timestamp: u64,
        inputs: Vec<Slip>,
        outputs: Vec<Slip>,
        message: Vec<u8>,
        transaction_type: TransactionType,
    ) -> Self {
        Self {
            timestamp,
            inputs,
            outputs,
            message,
            transaction_type,
        }
    }
}

impl Default for TransactionCore {
    fn default() -> Self {
        Self::new(
            create_timestamp(),
            vec![],
            vec![],
            vec![],
            TransactionType::Normal,
        )
    }
}

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Transaction {
    core: TransactionCore,
    #[serde_as(as = "[_; 64]")]
    signature: [u8; 64], // compact signatures are 64 bytes; DER signatures are 68-72 bytes
}

impl Transaction {
    pub fn new(core: TransactionCore, signature: [u8; 64]) -> Self {
        Self { core, signature }
    }

    pub fn add_input(&mut self, input_slip: Slip) {
        self.core.inputs.push(input_slip);
    }

    pub fn add_output(&mut self, output_slip: Slip) {
        self.core.outputs.push(output_slip);
    }

    pub fn get_timestamp(&self) -> u64 {
        self.core.timestamp
    }

    pub fn get_transaction_type(&self) -> TransactionType {
        self.core.transaction_type
    }

    pub fn get_inputs(&self) -> &Vec<Slip> {
        &self.core.inputs
    }

    pub fn get_outputs(&self) -> &Vec<Slip> {
        &self.core.outputs
    }

    pub fn get_message(&self) -> &Vec<u8> {
        &self.core.message
    }

    pub fn get_signature(&self) -> [u8; 64] {
        self.signature
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.core.timestamp = timestamp;
    }

    pub fn set_transaction_type(&mut self, transaction_type: TransactionType) {
        self.core.transaction_type = transaction_type;
    }

    pub fn set_message(&mut self, message: Vec<u8>) {
        self.core.message = message;
    }

    // TODO - this is just a stub

    pub fn set_signature(&self) -> SaitoSignature {
        [0; 64]
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new(TransactionCore::default(), [0; 64])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_core_default_test() {
        let timestamp = create_timestamp();
        let tx_core = TransactionCore::default();
        assert_eq!(tx_core.timestamp, timestamp);
        assert_eq!(tx_core.inputs, vec![]);
        assert_eq!(tx_core.outputs, vec![]);
        assert_eq!(tx_core.message, Vec::<u8>::new());
        assert_eq!(tx_core.transaction_type, TransactionType::Normal);
    }

    #[test]
    fn transaction_core_new_test() {
        let timestamp = create_timestamp();
        let tx_core =
            TransactionCore::new(timestamp, vec![], vec![], vec![], TransactionType::Normal);
        assert_eq!(tx_core.timestamp, timestamp);
        assert_eq!(tx_core.inputs, vec![]);
        assert_eq!(tx_core.outputs, vec![]);
        assert_eq!(tx_core.message, Vec::<u8>::new());
        assert_eq!(tx_core.transaction_type, TransactionType::Normal);
    }

    #[test]
    fn transaction_default_test() {
        let timestamp = create_timestamp();
        let tx = Transaction::default();
        assert_eq!(tx.core.timestamp, timestamp);
        assert_eq!(tx.core.inputs, vec![]);
        assert_eq!(tx.core.outputs, vec![]);
        assert_eq!(tx.core.message, Vec::<u8>::new());
        assert_eq!(tx.core.transaction_type, TransactionType::Normal);
        assert_eq!(tx.signature, [0; 64]);
    }

    #[test]
    fn transaction_new_test() {
        let timestamp = create_timestamp();
        let tx_core = TransactionCore::default();
        let tx = Transaction::new(tx_core, [0; 64]);
        assert_eq!(tx.core.timestamp, timestamp);
        assert_eq!(tx.core.inputs, vec![]);
        assert_eq!(tx.core.outputs, vec![]);
        assert_eq!(tx.core.message, Vec::<u8>::new());
        assert_eq!(tx.core.transaction_type, TransactionType::Normal);
        assert_eq!(tx.signature, [0; 64]);
    }
}
