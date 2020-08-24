use std::prelude::v1::Vec;

/// Transaction internal structure
pub mod data;
/// Transaction fee calculation
pub mod fee;
/// Witness structures (e.g. signatures) for transactions
pub mod witness;

use std::fmt;

use parity_scale_codec::{Decode, Encode, Error, Input, Output};

use self::data::Tx;
use self::witness::TxWitness;
use crate::mls::MLSHandshakeAux;
use crate::state::account::{
    DepositBondTx, StakedStateOpAttributes, StakedStateOpWitness, UnbondTx, UnjailTx,
    WithdrawUnbondedTx,
};
use crate::state::tendermint::BlockHeight;
use crate::state::validator::NodeJoinRequestTx;
use crate::tx::data::TxId;
use aead::Payload;
use data::input::{TxoPointer, TxoSize};
use data::output::TxOut;

/// Maximum (Tendermint-outer payload) transaction size
pub const TX_AUX_SIZE: usize = 1024 * 60; // 60 KB

/// wrapper around transactions with outputs
#[derive(Encode, Decode, Clone)]
pub enum TxWithOutputs {
    /// normal transfer
    Transfer(Tx),
    /// withdrawing unbonded amount from a staked state
    StakeWithdraw(WithdrawUnbondedTx),
}

impl TxWithOutputs {
    /// returns the particular transaction type's outputs
    pub fn outputs(&self) -> &[TxOut] {
        match self {
            TxWithOutputs::Transfer(tx) => &tx.outputs,
            TxWithOutputs::StakeWithdraw(tx) => &tx.outputs,
        }
    }

    /// returns the particular transaction type's id (currently blake3_hash(SCALE-encoded tx))
    pub fn id(&self) -> TxId {
        match self {
            TxWithOutputs::Transfer(tx) => tx.id(),
            TxWithOutputs::StakeWithdraw(tx) => tx.id(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
/// Plain transaction parts "visible" inside enclaves
pub enum PlainTxAux {
    /// both private; normal value transfer Tx with the vector of witnesses
    TransferTx(Tx, TxWitness),
    /// only the witness, as only "input" data are private
    DepositStakeTx(TxWitness),
    /// only the TX data / new outputs are private
    WithdrawUnbondedStakeTx(WithdrawUnbondedTx),
}

impl Encode for PlainTxAux {
    fn encode_to<EncOut: Output>(&self, dest: &mut EncOut) {
        match *self {
            PlainTxAux::TransferTx(ref tx, ref witness) => {
                dest.push_byte(0);
                dest.push(tx);
                dest.push(witness);
            }
            PlainTxAux::DepositStakeTx(ref witness) => {
                dest.push_byte(1);
                dest.push(witness);
            }
            PlainTxAux::WithdrawUnbondedStakeTx(ref tx) => {
                dest.push_byte(2);
                dest.push(tx);
            }
        }
    }

    fn size_hint(&self) -> usize {
        1 + match self {
            PlainTxAux::TransferTx(tx, witness) => tx.size_hint() + witness.size_hint(),
            PlainTxAux::DepositStakeTx(witness) => witness.size_hint(),
            PlainTxAux::WithdrawUnbondedStakeTx(tx) => tx.size_hint(),
        }
    }
}

impl Decode for PlainTxAux {
    fn decode<DecIn: Input>(input: &mut DecIn) -> Result<Self, Error> {
        let tag = input.read_byte()?;
        match tag {
            0 => {
                let tx = Tx::decode(input)?;
                let witness = TxWitness::decode(input)?;
                Ok(PlainTxAux::TransferTx(tx, witness))
            }
            1 => {
                let witness = TxWitness::decode(input)?;
                Ok(PlainTxAux::DepositStakeTx(witness))
            }
            2 => {
                let tx = WithdrawUnbondedTx::decode(input)?;
                Ok(PlainTxAux::WithdrawUnbondedStakeTx(tx))
            }
            _ => Err("No such variant in enum PlainTxAux".into()),
        }
    }
}

impl PlainTxAux {
    /// creates a new Tx with a vector of witnesses (mainly for testing/tools)
    pub fn new(tx: Tx, witness: TxWitness) -> Self {
        PlainTxAux::TransferTx(tx, witness)
    }
}

impl fmt::Display for PlainTxAux {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlainTxAux::TransferTx(tx, witness) => display_tx_witness(f, tx, witness),
            PlainTxAux::DepositStakeTx(witness) => writeln!(f, "witness: {:?}\n", witness),
            PlainTxAux::WithdrawUnbondedStakeTx(tx) => writeln!(f, "Tx:\n{}", tx),
        }
    }
}

/// plqin TX payload to be obfuscated
/// (a helper structure)
pub struct TxToObfuscate {
    /// the raw plain transaction payload
    /// PlainTxAux encoded using SCALE
    pub txpayload: Vec<u8>,
    /// the corresponding transaction ID
    pub txid: TxId,
}

impl TxToObfuscate {
    /// creates the helper structure (if txid matches)
    /// note: txid is needed to be provided, as the obfuscated payload
    /// of deposit tx only contains the witness payload
    pub fn from(tx: PlainTxAux, txid: TxId) -> Option<Self> {
        match &tx {
            PlainTxAux::TransferTx(itx, _) => {
                if itx.id() == txid {
                    Some(TxToObfuscate {
                        txpayload: tx.encode(),
                        txid,
                    })
                } else {
                    None
                }
            }
            PlainTxAux::DepositStakeTx(_) => Some(TxToObfuscate {
                txpayload: tx.encode(),
                txid,
            }),
            PlainTxAux::WithdrawUnbondedStakeTx(itx) => {
                if itx.id() == txid {
                    Some(TxToObfuscate {
                        txpayload: tx.encode(),
                        txid,
                    })
                } else {
                    None
                }
            }
        }
    }
}

impl<'tx> Into<Payload<'tx, 'tx>> for &'tx TxToObfuscate {
    fn into(self) -> Payload<'tx, 'tx> {
        Payload {
            msg: &self.txpayload,
            aad: &self.txid,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
/// obfuscated TX payload
pub struct TxObfuscated {
    /// to denote which key the payload was obfuscated with;
    /// refers to the height at which TDBE key rotation finalized
    pub key_from: BlockHeight,
    /// the "nonce" / IV to use with the obfuscation key
    pub init_vector: [u8; 12],
    /// AEAD transaction payload
    pub txpayload: Vec<u8>,
    /// transaction id -- to be used as authentication tag
    /// when working with the AEAD transaction payload
    pub txid: TxId,
}

impl Encode for TxObfuscated {
    fn encode_to<EncOut: Output>(&self, dest: &mut EncOut) {
        dest.push(&self.key_from);
        dest.push(&self.init_vector);
        dest.push(&self.txpayload);
        dest.push(&self.txid);
    }

    fn size_hint(&self) -> usize {
        self.key_from.size_hint()
            + self.init_vector.size_hint()
            + self.txpayload.len()
            + self.txid.size_hint()
    }
}

impl Decode for TxObfuscated {
    fn decode<DecIn: Input>(input: &mut DecIn) -> Result<Self, Error> {
        let key_from = BlockHeight::decode(input)?;
        let init_vector: [u8; 12] = Decode::decode(input)?;
        let txpayload: Vec<u8> = Vec::decode(input)?;
        let txid = TxId::decode(input)?;
        Ok(TxObfuscated {
            key_from,
            init_vector,
            txpayload,
            txid,
        })
    }
}

impl<'tx> Into<Payload<'tx, 'tx>> for &'tx TxObfuscated {
    fn into(self) -> Payload<'tx, 'tx> {
        Payload {
            msg: &self.txpayload,
            aad: &self.txid,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
/// transactions with parts that are only viewable inside TEE
pub enum TxEnclaveAux {
    /// normal value transfer Tx with the vector of witnesses
    TransferTx {
        /// inputs that the transaction spends (public information)
        inputs: Vec<TxoPointer>,
        /// outputs to be created if valid
        no_of_outputs: TxoSize,
        /// the payload that can only be validated inside TEE that's synced up with the network
        payload: TxObfuscated,
    },
    /// Tx "spends" utxos to be deposited as bonded stake in an account (witnesses as in transfer)
    DepositStakeTx {
        /// most of the information is public here, e.g. what address it is depositing to
        tx: DepositBondTx,
        /// the payload that can only be validated inside TEE that's synced up with the network
        /// here, it's only the "witness"
        payload: TxObfuscated,
    },
    /// Tx that "creates" utxos out of account state; withdraws unbonded stake (witness for account)
    WithdrawUnbondedStakeTx {
        /// outputs to be created if valid
        no_of_outputs: TxoSize,
        /// witness for the corresponding staking address
        witness: StakedStateOpWitness,
        /// the payload that can only be validated inside TEE that's synced up with the network
        /// here's the transaction outputs + attributes
        payload: TxObfuscated,
    },
}

impl Encode for TxEnclaveAux {
    fn encode_to<EncOut: Output>(&self, dest: &mut EncOut) {
        match *self {
            TxEnclaveAux::TransferTx {
                ref inputs,
                ref no_of_outputs,
                ref payload,
            } => {
                dest.push_byte(0);
                dest.push(inputs);
                dest.push(no_of_outputs);
                dest.push(payload);
            }
            TxEnclaveAux::DepositStakeTx {
                ref tx,
                ref payload,
            } => {
                dest.push_byte(1);
                dest.push(tx);
                dest.push(payload);
            }
            TxEnclaveAux::WithdrawUnbondedStakeTx {
                ref no_of_outputs,
                ref witness,
                ref payload,
            } => {
                dest.push_byte(2);
                dest.push(no_of_outputs);
                dest.push(witness);
                dest.push(payload);
            }
        }
    }

    fn size_hint(&self) -> usize {
        1 + match self {
            TxEnclaveAux::TransferTx {
                ref inputs,
                ref payload,
                ..
            } => inputs.size_hint() + 2 + payload.size_hint(),
            TxEnclaveAux::DepositStakeTx {
                ref tx,
                ref payload,
            } => tx.size_hint() + payload.size_hint(),
            TxEnclaveAux::WithdrawUnbondedStakeTx {
                ref witness,
                ref payload,
                ..
            } => witness.size_hint() + 2 + payload.size_hint(),
        }
    }
}

impl Decode for TxEnclaveAux {
    fn decode<DecIn: Input>(input: &mut DecIn) -> Result<Self, Error> {
        let tag = input.read_byte()?;
        // note: 3.. tags expected for TDBE tx (MLS messages)
        match tag {
            0 => {
                let inputs: Vec<TxoPointer> = Vec::decode(input)?;
                let no_of_outputs = TxoSize::decode(input)?;
                let payload = TxObfuscated::decode(input)?;
                Ok(TxEnclaveAux::TransferTx {
                    inputs,
                    no_of_outputs,
                    payload,
                })
            }
            1 => {
                let tx = DepositBondTx::decode(input)?;
                let payload = TxObfuscated::decode(input)?;
                Ok(TxEnclaveAux::DepositStakeTx { tx, payload })
            }
            2 => {
                let no_of_outputs = TxoSize::decode(input)?;
                let witness = StakedStateOpWitness::decode(input)?;
                let payload = TxObfuscated::decode(input)?;
                Ok(TxEnclaveAux::WithdrawUnbondedStakeTx {
                    no_of_outputs,
                    witness,
                    payload,
                })
            }
            _ => Err("No such variant in enum TxEnclaveAux".into()),
        }
    }
}

impl TxEnclaveAux {
    /// retrieves a TX ID (of plaintxaux if relevant -- blake3(<tx type tag> || scale_codec_bytes(tx)))
    pub fn tx_id(&self) -> TxId {
        match self {
            TxEnclaveAux::TransferTx {
                payload: TxObfuscated { txid, .. },
                ..
            } => *txid,
            TxEnclaveAux::DepositStakeTx { tx, .. } => tx.id(),
            TxEnclaveAux::WithdrawUnbondedStakeTx {
                payload: TxObfuscated { txid, .. },
                ..
            } => *txid,
        }
    }
}

/// Transactions that are directly processed in non-enclave execution environment (chain-abci)
/// TODO/NOTE: other TX types expected -- update of council node metadata, bonus donation, ...
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TxPublicAux {
    /// Tx that modifies staked state -- moves some bonded stake into unbonded (witness for staked state)
    UnbondStakeTx(UnbondTx, StakedStateOpWitness),
    /// Tx that unjails a staked state
    UnjailTx(UnjailTx, StakedStateOpWitness),
    /// Tx that updates a staked state with node (community or council node) details
    NodeJoinTx(NodeJoinRequestTx, StakedStateOpWitness),
}

impl Encode for TxPublicAux {
    fn encode_to<EncOut: Output>(&self, dest: &mut EncOut) {
        match *self {
            TxPublicAux::UnbondStakeTx(ref tx, ref witness) => {
                dest.push_byte(0);
                dest.push(tx);
                dest.push(witness);
            }
            TxPublicAux::UnjailTx(ref tx, ref witness) => {
                dest.push_byte(1);
                dest.push(tx);
                dest.push(witness);
            }
            TxPublicAux::NodeJoinTx(ref tx, ref witness) => {
                dest.push_byte(2);
                dest.push(tx);
                dest.push(witness);
            }
        }
    }

    fn size_hint(&self) -> usize {
        1 + match self {
            TxPublicAux::UnbondStakeTx(tx, witness) => tx.size_hint() + witness.size_hint(),
            TxPublicAux::UnjailTx(tx, witness) => tx.size_hint() + witness.size_hint(),
            TxPublicAux::NodeJoinTx(tx, witness) => tx.size_hint() + witness.size_hint(),
        }
    }
}

impl Decode for TxPublicAux {
    fn decode<DecIn: Input>(input: &mut DecIn) -> Result<Self, Error> {
        let tag = input.read_byte()?;
        // note: 3.. tags reserved for other tx types (node metadata update etc.)
        match tag {
            0 => {
                let tx = UnbondTx::decode(input)?;
                let witness = StakedStateOpWitness::decode(input)?;
                Ok(TxPublicAux::UnbondStakeTx(tx, witness))
            }
            1 => {
                let tx = UnjailTx::decode(input)?;
                let witness = StakedStateOpWitness::decode(input)?;
                Ok(TxPublicAux::UnjailTx(tx, witness))
            }
            2 => {
                let tx = NodeJoinRequestTx::decode(input)?;
                let witness = StakedStateOpWitness::decode(input)?;
                Ok(TxPublicAux::NodeJoinTx(tx, witness))
            }
            _ => Err("No such variant in enum TxPublicAux".into()),
        }
    }
}

impl TxPublicAux {
    /// retrieves a TX ID (currently blake3(<tx type tag> || scale_codec_bytes(tx)))
    pub fn tx_id(&self) -> TxId {
        match self {
            TxPublicAux::UnbondStakeTx(tx, _) => tx.id(),
            TxPublicAux::UnjailTx(tx, _) => tx.id(),
            TxPublicAux::NodeJoinTx(tx, _) => tx.id(),
        }
    }

    /// returns the transaction attributes (containing version, network identifier...s)
    pub fn attributes(&self) -> &StakedStateOpAttributes {
        match self {
            TxPublicAux::UnbondStakeTx(tx, _) => &tx.attributes,
            TxPublicAux::UnjailTx(tx, _) => &tx.attributes,
            TxPublicAux::NodeJoinTx(tx, _) => &tx.attributes,
        }
    }

    /// Get chain hex id, only works on public tx.
    pub fn chain_hex_id(&self) -> u8 {
        self.attributes().chain_hex_id
    }
}

/// Outer transaction type (broadcast in Tendermint tx payloads)
///
/// # TX format evolution
/// - If the change is more or less the same behaviour,
/// it can be done on a particular tx type component
/// -- e.g. if there's a new way to lock transaction output,
/// it can be a variant in ExtendedAddr + a corresponding witness type.
/// (could be even to e.g. support a different signature scheme)
/// - If the extension is a different behaviour, it'll be a new transaction type (possibly under enclave or public auxiliary type).
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TxAux {
    /// transactions that need to be processed inside TEE (or need TEE in their finalization)
    EnclaveTx(TxEnclaveAux),
    /// transactions that are processed directly in untrusted environment (chain-abci)
    PublicTx(TxPublicAux),
    /// wrappers around TDBE-related MLS handshake messages being broadcasted
    MLSHandshake(MLSHandshakeAux),
}

impl Encode for TxAux {
    fn encode_to<EncOut: Output>(&self, dest: &mut EncOut) {
        match *self {
            TxAux::EnclaveTx(ref tx) => {
                dest.push_byte(0);
                dest.push(tx);
            }
            TxAux::PublicTx(ref tx) => {
                dest.push_byte(1);
                dest.push(tx);
            }
            TxAux::MLSHandshake(ref tx) => {
                dest.push_byte(2);
                dest.push(tx);
            }
        }
    }

    fn size_hint(&self) -> usize {
        1 + match self {
            TxAux::EnclaveTx(tx) => tx.size_hint(),
            TxAux::PublicTx(tx) => tx.size_hint(),
            TxAux::MLSHandshake(tx) => tx.size_hint(),
        }
    }
}

impl Decode for TxAux {
    fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
        let size = input
            .remaining_len()?
            .ok_or_else(|| "Unable to calculate size of input")?;

        if size > TX_AUX_SIZE {
            return Err("Input too large".into());
        }

        match input.read_byte()? {
            0 => Ok(TxAux::EnclaveTx(TxEnclaveAux::decode(input)?)),
            1 => Ok(TxAux::PublicTx(TxPublicAux::decode(input)?)),
            2 => Ok(TxAux::MLSHandshake(MLSHandshakeAux::decode(input)?)),
            _ => Err("No such variant in enum TxAux".into()),
        }
    }
}

/// This trait is for transaction data type to define the "txid"
/// that's used as a message digest in signing.
/// *IMPORTANT*: only auto-implement TransactionId for data types
/// with transaction data, not the outer type that includes the witness data.
/// For the outer type, you can use a custom function (like `tx_id`)
/// or make sure you override the default implementation to take the identifier
/// from the transaction data type.
#[cfg(feature = "new-txid")]
pub trait TransactionId {
    /// retrieves a TX ID (currently blake3(<tx type tag> || scale_codec_bytes(tx)))
    fn id(&self) -> TxId;
}

#[cfg(feature = "new-txid")]
impl<T: Into<TaggedTransaction> + Clone> TransactionId for T {
    fn id(&self) -> TxId {
        let tx: TaggedTransaction = self.clone().into();
        TaggedTransaction::from(tx).id()
    }
}

/// 0.5-compatible version: This trait is for transaction data type to define the "txid"
/// that's used as a message digest in signing.
#[cfg(not(feature = "new-txid"))]
pub trait TransactionId: Encode {
    /// 0.5-compatible version: retrieves a TX ID (currently blake3(scale_codec_bytes(tx)))
    fn id(&self) -> TxId {
        blake3::hash(&self.encode()).into()
    }
}

/// used for TXID calculation -- contains all possible tx types
/// NOTE: do not reorder, as the byte tag is used in txid calculation
#[cfg(feature = "new-txid")]
#[derive(Encode)]
pub enum TaggedTransaction {
    /// transfer transaction
    Transfer(Tx),
    /// deposit stake to bonded amount
    Deposit(DepositBondTx),
    /// withdraw unbonded amount
    Withdraw(WithdrawUnbondedTx),
    /// unbond stake
    UnbondStakeTx(UnbondTx),
    /// unjail request
    UnjailTx(UnjailTx),
    /// node join request
    NodeJoinTx(NodeJoinRequestTx),
    /// removal proposals + commit
    MLSRemoveCommitProposal(crate::mls::CommitRemoveTx),
    /// update proposal + commit
    MLSSelfUpdateProposal(crate::mls::SelfUpdateProposalTx),
    /// NACK
    MLSMsgNack(crate::mls::NackMsgTx),
}

#[cfg(feature = "new-txid")]
impl TaggedTransaction {
    fn id(&self) -> TxId {
        blake3::hash(&self.encode()).into()
    }
}

impl TxAux {
    /// retrieves a TX ID (currently blake3(<tx type tag> || scale_codec_bytes(tx)))
    pub fn tx_id(&self) -> TxId {
        match self {
            TxAux::EnclaveTx(tx) => tx.tx_id(),
            TxAux::PublicTx(tx) => tx.tx_id(),
            TxAux::MLSHandshake(tx) => tx.tx_id(),
        }
    }
}

fn display_tx_witness<T: fmt::Display, W: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    tx: T,
    witness: W,
) -> fmt::Result {
    writeln!(f, "Tx:\n{}", tx)?;
    writeln!(f, "witness: {:?}\n", witness)
}

impl fmt::Display for TxAux {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self {
            TxAux::EnclaveTx(TxEnclaveAux::TransferTx {
                payload: TxObfuscated { txid, .. },
                inputs,
                ..
            }) => {
                writeln!(f, "Transfer Tx id:\n{}", hex::encode(&txid[..]))?;
                writeln!(f, "tx inputs: {:?}\n", inputs)
            }
            TxAux::EnclaveTx(TxEnclaveAux::DepositStakeTx { tx, .. }) => writeln!(f, "Tx:\n{}", tx),
            TxAux::PublicTx(TxPublicAux::UnbondStakeTx(tx, witness)) => {
                display_tx_witness(f, tx, witness)
            }
            TxAux::EnclaveTx(TxEnclaveAux::WithdrawUnbondedStakeTx {
                payload: TxObfuscated { txid, .. },
                witness,
                ..
            }) => {
                writeln!(
                    f,
                    "Withdraw Unbonded Stake Tx id:\n{}",
                    hex::encode(&txid[..])
                )?;
                writeln!(f, "witness: {:?}\n", witness)
            }
            TxAux::PublicTx(TxPublicAux::UnjailTx(tx, witness)) => {
                display_tx_witness(f, tx, witness)
            }
            TxAux::PublicTx(TxPublicAux::NodeJoinTx(tx, witness)) => {
                display_tx_witness(f, tx, witness)
            }
            TxAux::MLSHandshake(_) => {
                // FIXME
                writeln!(f, "mls handshake")
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::common::MerkleTree;
    use crate::init::coin::Coin;
    use crate::tx::data::access::{TxAccess, TxAccessPolicy};
    use crate::tx::data::address::ExtendedAddr;
    use crate::tx::data::input::TxoPointer;
    use crate::tx::data::output::TxOut;
    use crate::tx::witness::tree::RawXOnlyPubkey;
    use crate::tx::witness::TxInWitness;
    use parity_scale_codec::{Decode, Encode};
    use secp256k1::{key::XOnlyPublicKey, schnorrsig::schnorr_sign, Message, PublicKey, SecretKey};

    // TODO: rewrite as quickcheck prop
    #[test]
    fn encode_decode() {
        // not a valid transaction, only to test enconding-decoding
        let mut tx = Tx::new();
        tx.add_input(TxoPointer::new([0x01; 32], 1));
        tx.add_output(TxOut::new(ExtendedAddr::OrTree([0xbb; 32]), Coin::unit()));
        let secp = secp256k1::SECP256K1;
        let sk1 = SecretKey::from_slice(&[0xcc; 32][..]).expect("secret key");
        let pk1 = PublicKey::from_secret_key(&secp, &sk1);
        let raw_pk1 = RawXOnlyPubkey::from(XOnlyPublicKey::from_pubkey(&pk1).0.serialize());

        let raw_public_keys = vec![raw_pk1];

        tx.attributes
            .allowed_view
            .push(TxAccessPolicy::new(pk1, TxAccess::AllData));

        let msg = Message::from_slice(&tx.id()).expect("msg");

        let merkle = MerkleTree::new(raw_public_keys.clone());

        let w1 = TxInWitness::TreeSig(
            schnorr_sign(&secp, &msg, &sk1, &mut rand::thread_rng()),
            merkle.generate_proof(raw_public_keys[0].clone()).unwrap(),
        );
        let txa = PlainTxAux::TransferTx(tx, vec![w1].into());
        let mut encoded: Vec<u8> = txa.encode();
        let mut data: &[u8] = encoded.as_mut();
        let decoded = PlainTxAux::decode(&mut data).expect("decode tx aux");
        assert_eq!(txa, decoded);
    }
}
