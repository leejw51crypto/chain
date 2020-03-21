use indexmap::IndexSet;
use parity_scale_codec::{Decode, Encode, Input, Output};

use crate::service::{load_wallet_state, WalletState};
use chain_core::common::H256;
use chain_core::init::address::RedeemAddress;
use chain_core::state::account::StakedStateAddress;
use chain_core::tx::data::address::ExtendedAddr;
use client_common::{
    Error, ErrorKind, PrivateKey, PublicKey, Result, ResultExt, SecKey, SecureStorage, Storage,
};
use std::fmt;
//use serde::ser::{Serialize, SerializeStruct, Serializer};
use parity_scale_codec::alloc::collections::BTreeMap;
use secstr::SecUtf8;
use serde::de::{self, Visitor};
use serde::export::PhantomData;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str;
/// Key space of wallet
const KEYSPACE: &str = "core_wallet";

fn get_public_keyspace(name: &str) -> String {
    format!("{}_{}_publickey", KEYSPACE, name)
}

fn get_stakingkeyset_keyspace(name: &str) -> String {
    format!("{}_{}_stakingkey_set", KEYSPACE, name)
}

fn get_private_keyspace(name: &str) -> String {
    format!("{}_{}_privatekey", KEYSPACE, name)
}

fn get_roothashset_keyspace(name: &str) -> String {
    format!("{}_{}_roothash_set", KEYSPACE, name)
}

pub fn get_multisig_keyspace(name: &str) -> String {
    format!("{}_{}_multisigaddress", KEYSPACE, name)
}

fn get_info_keyspace(name: &str) -> String {
    format!("{}_{}_info", KEYSPACE, name)
}

fn get_wallet_keyspace() -> String {
    format!("{}_walletname", KEYSPACE)
}

fn serde_to_str<T, S>(value: &T, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    T: Encode,
    S: Serializer,
{
    let value_str = base64::encode(&value.encode());
    serializer.serialize_str(&value_str)
}

fn deserde_from_str<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Decode,
{
    struct Helper<S>(PhantomData<S>);

    impl<'de, S> Visitor<'de> for Helper<S>
    where
        S: Decode,
    {
        type Value = S;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "expect valid str")
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            let raw_data = base64::decode(value).map_err(de::Error::custom)?;
            let v = Self::Value::decode(&mut raw_data.as_slice()).map_err(de::Error::custom)?;
            Ok(v)
        }
    }
    deserializer.deserialize_str(Helper(PhantomData))
}

/// Wallet information to export and import
#[derive(Debug, Deserialize, Serialize)]
pub struct WalletInfo {
    /// name of the the wallet
    pub name: String,
    /// wallet meta data
    #[serde(deserialize_with = "deserde_from_str", serialize_with = "serde_to_str")]
    pub wallet: Wallet,
    /// private key of the wallet
    #[serde(deserialize_with = "deserde_from_str", serialize_with = "serde_to_str")]
    pub private_key: PrivateKey,
    /// passphrase used when import wallet
    pub passphrase: Option<SecUtf8>,
}

/// Wallet meta data
#[derive(Debug, Clone)]
pub struct Wallet {
    /// view key to decrypt enclave transactions
    pub view_key: PublicKey,
    /// public key and private key pair
    pub key_pairs: BTreeMap<PublicKey, PrivateKey>,
    /// public keys to construct transfer addresses
    pub public_keys: IndexSet<PublicKey>,
    /// public keys of staking addresses
    pub staking_keys: IndexSet<PublicKey>,
    /// root hashes of multi-sig transfer addresses
    pub root_hashes: IndexSet<H256>,
}

impl Encode for Wallet {
    fn encode_to<W: Output>(&self, dest: &mut W) {
        self.view_key.encode_to(dest);
    }
}

impl Decode for Wallet {
    fn decode<I: Input>(input: &mut I) -> std::result::Result<Self, parity_scale_codec::Error> {
        let view_key = PublicKey::decode(input)?;
        let key_pairs = BTreeMap::new();
        let public_keys = IndexSet::new();
        let staking_keys = IndexSet::new();
        let root_hashes = IndexSet::new();

        Ok(Wallet {
            view_key,
            key_pairs,
            public_keys,
            staking_keys,
            root_hashes,
        })
    }
}

impl Wallet {
    /// Creates a new instance of `Wallet`
    pub fn new(view_key: PublicKey) -> Self {
        Self {
            view_key,
            key_pairs: Default::default(),
            public_keys: Default::default(),
            staking_keys: Default::default(),
            root_hashes: Default::default(),
        }
    }

    /// Returns all staking addresses stored in a wallet
    pub fn staking_addresses(&self) -> IndexSet<StakedStateAddress> {
        self.staking_keys
            .iter()
            .map(|public_key| StakedStateAddress::BasicRedeem(RedeemAddress::from(public_key)))
            .collect()
    }

    /// Returns all tree addresses stored in a wallet
    pub fn transfer_addresses(&self) -> IndexSet<ExtendedAddr> {
        self.root_hashes
            .iter()
            .cloned()
            .map(ExtendedAddr::OrTree)
            .collect()
    }
}

fn read_pubkey<S: SecureStorage>(storage: &S, keyspace: &str, key: &str) -> Result<PublicKey> {
    let value = storage.get(keyspace, key)?;
    if let Some(raw_value) = value {
        let pubkey = PublicKey::deserialize_from(&raw_value)?;
        Ok(pubkey)
    } else {
        Err(Error::new(ErrorKind::InvalidInput, "read pubkey error"))
    }
}

fn write_pubkey<S: SecureStorage>(
    storage: &S,
    keyspace: &str,
    key: &str,
    value: &PublicKey,
) -> Result<()> {
    storage
        .set(keyspace, key, value.serialize())
        .expect("write pubkey");
    Ok(())
}

fn read_string<S: SecureStorage>(storage: &S, keyspace: &str, key: &str) -> Result<String> {
    let value = storage.get(keyspace, key.as_bytes())?;
    if let Some(raw_value) = value {
        let ret = str::from_utf8(&raw_value).unwrap();
        Ok(ret.to_string())
    } else {
        Err(Error::new(ErrorKind::InvalidInput, "read string error"))
    }
}

/// Load wallet from storage
pub fn load_wallet<S: SecureStorage>(
    storage: &S,
    name: &str,
    enckey: &SecKey,
) -> Result<Option<Wallet>> {
    let wallet: Option<Wallet> = storage.load_secure(KEYSPACE, name, enckey)?;

    if let Some(value) = wallet {
        let mut new_wallet = value;
        // storage -> wallet
        let info_keyspace = get_info_keyspace(name);
        new_wallet.view_key = read_pubkey(storage, &info_keyspace, "viewkey")?;

        // pubkey
        let public_keyspace = get_public_keyspace(name);
        let private_keyspace = get_private_keyspace(name);
        let public_keys = storage.keys(&public_keyspace)?;
        for key in &public_keys {
            let pubkey = read_pubkey(storage, &public_keyspace, &str::from_utf8(&key).unwrap())?;
            new_wallet.public_keys.insert(pubkey.clone());

            if let Ok(value) =
                storage.get_secure(private_keyspace.clone(), pubkey.serialize(), enckey)
            {
                if let Some(raw_value) = value {
                    let privatekey = PrivateKey::deserialize_from(&raw_value).unwrap();
                    new_wallet.key_pairs.insert(pubkey, privatekey);
                }
            }
        }

        let staking_keyspace = get_stakingkeyset_keyspace(name);
        let staking_keys = storage.keys(&staking_keyspace)?;
        for key in &staking_keys {
            let stakingkey =
                read_pubkey(storage, &staking_keyspace, &str::from_utf8(&key).unwrap())?;
            new_wallet.staking_keys.insert(stakingkey);
        }

        // roothash
        let roothash_keyspace = get_roothashset_keyspace(name);
        let roothash_keys = storage.keys(&roothash_keyspace)?;
        for key in &roothash_keys {
            let value = storage.get(&roothash_keyspace, &key)?;
            if let Some(raw_value) = value {
                let mut roothash_found: H256 = H256::default();
                roothash_found.copy_from_slice(&raw_value);
                new_wallet.root_hashes.insert(roothash_found);
            }
        }
        return Ok(Some(new_wallet));
    }

    Ok(None)
}

/// Maintains mapping `wallet-name -> wallet-details`
#[derive(Debug, Default, Clone)]
pub struct WalletService<T: Storage> {
    storage: T,
}

impl<T> WalletService<T>
where
    T: Storage,
{
    /// Creates a new instance of wallet service
    pub fn new(storage: T) -> Self {
        WalletService { storage }
    }

    /// Get the wallet from storage
    pub fn get_wallet(&self, name: &str, enckey: &SecKey) -> Result<Wallet> {
        load_wallet(&self.storage, name, enckey)?.err_kind(ErrorKind::InvalidInput, || {
            format!("Wallet with name ({}) not found", name)
        })
    }

    /// Get the wallet state from storage
    // storage -> wallet
    pub fn get_wallet_state(&self, name: &str, enckey: &SecKey) -> Result<WalletState> {
        load_wallet_state(&self.storage, name, enckey)?.err_kind(ErrorKind::InvalidInput, || {
            format!("WalletState with name ({}) not found", name)
        })
    }

    /// Save wallet to storage
    pub fn save_wallet(&self, name: &str, enckey: &SecKey, wallet: &Wallet) -> Result<()> {
        self.storage
            .save_secure(KEYSPACE, name, enckey, wallet)
            .expect("save walet-name in save_wallet");

        let info_keyspace = get_info_keyspace(name);
        // write viewkey
        write_pubkey(&self.storage, &info_keyspace, "viewkey", &wallet.view_key)
            .expect("write_pubkey in save_wallet");

        for (pubkey, prikey) in &wallet.key_pairs {
            self.add_key_pairs(&name, &enckey, &pubkey, &prikey)?
        }

        // pubkey
        for public_key in wallet.public_keys.iter() {
            self.add_public_key(name, enckey, public_key)
                .expect("add_public_key in save_wallet");
        }

        // stakingkey
        for public_key in wallet.staking_keys.iter() {
            self.add_staking_key(name, enckey, public_key)
                .expect("add_staking_key in save_wallet");
        }

        // root hash
        for root_hash in wallet.root_hashes.iter() {
            self.add_root_hash(name, enckey, root_hash.clone())
                .expect("add root_hash in save_wallet");
        }

        Ok(())
    }

    /// Store the wallet to storage
    // wallet -> storage
    pub fn set_wallet(&self, name: &str, enckey: &SecKey, wallet: Wallet) -> Result<()> {
        self.save_wallet(name, enckey, &wallet)
    }

    /// Finds staking key corresponding to given redeem address
    pub fn find_staking_key(
        &self,
        name: &str,
        _enckey: &SecKey,
        redeem_address: &RedeemAddress,
    ) -> Result<Option<PublicKey>> {
        let stakingkeyset_keyspace = get_stakingkeyset_keyspace(name);

        if let Ok(value) = read_pubkey(
            &self.storage,
            &stakingkeyset_keyspace,
            &redeem_address.to_string(),
        ) {
            Ok(Some(value))
        } else {
            Err(Error::new(ErrorKind::InvalidInput, "finding staking"))
        }
    }

    /// Finds private_key corresponding to given public_key
    pub fn find_private_key(
        &self,
        name: &str,
        enckey: &SecKey,
        public_key: &PublicKey,
    ) -> Result<Option<PrivateKey>> {
        let private_keyspace = get_private_keyspace(name);

        // key: public_key
        // value: private_key
        let value = self
            .storage
            .get_secure(private_keyspace, public_key.serialize(), enckey)?;
        if let Some(raw_value) = value {
            let privatekey = PrivateKey::deserialize_from(&raw_value)?;
            Ok(Some(privatekey))
        } else {
            Err(Error::new(ErrorKind::InvalidInput, "private_key not found"))
        }
    }

    /// Checks if root hash exists in current wallet and returns root hash if exists
    pub fn find_root_hash(
        &self,
        name: &str,
        _enckey: &SecKey,
        address: &ExtendedAddr,
    ) -> Result<Option<H256>> {
        match address {
            ExtendedAddr::OrTree(ref root_hash) => {
                // roothashset
                let roothashset_keyspace = get_roothashset_keyspace(name);

                let value = self.storage.get(roothashset_keyspace, root_hash.to_vec())?;

                if let Some(raw_value) = value {
                    let mut roothash_found: H256 = H256::default();
                    roothash_found.copy_from_slice(&raw_value);

                    return Ok(Some(roothash_found));
                }
            }
        }

        Err(Error::new(ErrorKind::InvalidInput, "private_key not found"))
    }

    /// Creates a new wallet and returns wallet ID
    pub fn create(&self, name: &str, enckey: &SecKey, view_key: PublicKey) -> Result<()> {
        if self.storage.contains_key(KEYSPACE, name)? {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Wallet with name ({}) already exists", name),
            ));
        }

        self.set_wallet(name, enckey, Wallet::new(view_key.clone()))
            .expect("set_wallet in create wallet");

        let info_keyspace = get_info_keyspace(name);
        // key: "viewkey"
        // value: view-key
        write_pubkey(&self.storage, &info_keyspace, "viewkey", &view_key)?;

        // key: index
        // value: walletname
        let wallet_keyspace = get_wallet_keyspace();
        self.storage
            .set(wallet_keyspace, name, name.as_bytes().to_vec())?;

        Ok(())
    }

    /// Returns view key of wallet
    pub fn view_key(&self, name: &str, _enckey: &SecKey) -> Result<PublicKey> {
        let info_keyspace = get_info_keyspace(name);
        read_pubkey(&self.storage, &info_keyspace, "viewkey")
    }

    /// Returns all public keys stored in a wallet
    pub fn public_keys(&self, name: &str, _enckey: &SecKey) -> Result<IndexSet<PublicKey>> {
        if !self.storage.contains_key(KEYSPACE, name)? {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Wallet with name ({}) not exists in public_keys", name),
            ));
        }

        let public_keyspace = get_public_keyspace(name);
        let mut ret: IndexSet<PublicKey> = IndexSet::<PublicKey>::new();
        let keys = self.storage.keys(&public_keyspace)?;
        println!("keys=={}", keys.len());
        for key in keys {
            let pubkey = read_pubkey(
                &self.storage,
                &public_keyspace,
                &str::from_utf8(&key).unwrap(),
            )?;
            ret.insert(pubkey);
        }
        Ok(ret)
    }

    /// Returns all public keys corresponding to staking addresses stored in a wallet
    pub fn staking_keys(&self, name: &str, _enckey: &SecKey) -> Result<IndexSet<PublicKey>> {
        if !self.storage.contains_key(KEYSPACE, name)? {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Wallet with name ({}) not exists in staking_keys", name),
            ));
        }

        let stakingkey_keyspace = get_stakingkeyset_keyspace(name);
        let mut ret: IndexSet<PublicKey> = IndexSet::<PublicKey>::new();
        let keys = self.storage.keys(&stakingkey_keyspace)?;
        for key in keys {
            let pubkey = read_pubkey(
                &self.storage,
                &stakingkey_keyspace,
                &str::from_utf8(&key).unwrap(),
            )?;
            ret.insert(pubkey);
        }
        Ok(ret)
    }

    /// Returns all multi-sig addresses stored in a wallet
    pub fn root_hashes(&self, name: &str, _enckey: &SecKey) -> Result<IndexSet<H256>> {
        if !self.storage.contains_key(KEYSPACE, name)? {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Wallet with name ({}) not exists in roothashes", name),
            ));
        }
        let roothash_keyspace = get_roothashset_keyspace(name);
        let mut ret: IndexSet<H256> = IndexSet::<H256>::new();
        let keys = self.storage.keys(&roothash_keyspace)?;
        for key in keys {
            let value = self.storage.get(&roothash_keyspace, &key)?;
            if let Some(raw_value) = value {
                let mut roothash_found: H256 = H256::default();
                roothash_found.copy_from_slice(&raw_value);
                ret.insert(roothash_found);
            }
        }
        Ok(ret)
    }

    /// Returns all staking addresses stored in a wallet
    pub fn staking_addresses(
        &self,
        name: &str,
        _enckey: &SecKey,
    ) -> Result<IndexSet<StakedStateAddress>> {
        let stakingkey_keyspace = get_stakingkeyset_keyspace(name);

        let mut ret: IndexSet<StakedStateAddress> = IndexSet::<StakedStateAddress>::new();
        let keys = self.storage.keys(&stakingkey_keyspace)?;
        for key in keys {
            let pubkey = read_pubkey(
                &self.storage,
                &stakingkey_keyspace,
                &str::from_utf8(&key).unwrap(),
            )?;
            let staked = StakedStateAddress::BasicRedeem(RedeemAddress::from(&pubkey));
            ret.insert(staked);
        }
        Ok(ret)
    }

    /// Returns all tree addresses stored in a wallet
    pub fn transfer_addresses(
        &self,
        name: &str,
        _enckey: &SecKey,
    ) -> Result<IndexSet<ExtendedAddr>> {
        let roothash_keyspace = get_roothashset_keyspace(name);

        let mut ret: IndexSet<ExtendedAddr> = IndexSet::<ExtendedAddr>::new();
        let keys = self.storage.keys(&roothash_keyspace)?;
        for key in &keys {
            let value = self.storage.get(&roothash_keyspace, &key)?;
            if let Some(raw_value) = value {
                let mut roothash_found: H256 = H256::default();
                roothash_found.copy_from_slice(&raw_value);
                let extended_addr = ExtendedAddr::OrTree(roothash_found);
                ret.insert(extended_addr);
            }
        }

        Ok(ret)
    }

    /// Adds a (public_key, private_key) pair to given wallet
    pub fn add_key_pairs(
        &self,
        name: &str,
        enckey: &SecKey,
        public_key: &PublicKey,
        private_key: &PrivateKey,
    ) -> Result<()> {
        let private_keyspace = get_private_keyspace(name);

        // key: public_key
        // value: private_key
        self.storage.set_secure(
            private_keyspace,
            public_key.serialize(),
            private_key.serialize(),
            enckey,
        )?;
        Ok(())
    }

    /// Adds a public key to given wallet
    pub fn add_public_key(
        &self,
        name: &str,
        _enckey: &SecKey,
        public_key: &PublicKey,
    ) -> Result<()> {
        let public_keyspace = get_public_keyspace(name);

        // key: index
        // value: publickey
        write_pubkey(
            &self.storage,
            &public_keyspace,
            &hex::encode(&public_key.serialize()),
            &public_key,
        )
        .expect("write pubkey in add_public_key");

        Ok(())
    }

    /// Adds a public key corresponding to a staking address to given wallet
    pub fn add_staking_key(
        &self,
        name: &str,
        _enckey: &SecKey,
        staking_key: &PublicKey,
    ) -> Result<()> {
        // stakingkey set
        // key: redeem address (20 bytes)
        // value: staking key (<-publickey)
        let redeemaddress = RedeemAddress::from(staking_key).to_string();
        let stakingkeyset_keyspace = get_stakingkeyset_keyspace(name);

        write_pubkey(
            &self.storage,
            &stakingkeyset_keyspace,
            &redeemaddress,
            &staking_key,
        )
        .expect("write pubkey");

        Ok(())
    }

    /// Adds a multi-sig address to given wallet
    pub fn add_root_hash(&self, name: &str, _enckey: &SecKey, root_hash: H256) -> Result<()> {
        // roothashset
        let roothashset_keyspace = get_roothashset_keyspace(name);
        self.storage
            .set(roothashset_keyspace, root_hash.to_vec(), root_hash.to_vec())?;

        Ok(())
    }

    /// Retrieves names of all the stored wallets
    pub fn names(&self) -> Result<Vec<String>> {
        let wallet_keyspace = get_wallet_keyspace();
        let keys = self.storage.keys(&wallet_keyspace)?;
        let mut names: Vec<String> = vec![];
        for key in keys {
            let string_key = String::from_utf8(key).chain(|| {
                (
                    ErrorKind::DeserializationError,
                    "Unable to deserialize wallet names in storage",
                )
            })?;
            let name_found = read_string(&self.storage, &wallet_keyspace, &string_key)?;
            names.push(name_found);
        }
        Ok(names)
    }

    /// Clears all storage
    pub fn clear(&self) -> Result<()> {
        let wallet_keyspace = get_wallet_keyspace();
        let keys = self.storage.keys(&wallet_keyspace)?;
        for key in keys {
            let string_key = String::from_utf8(key).chain(|| {
                (
                    ErrorKind::DeserializationError,
                    "Unable to deserialize wallet names in storage",
                )
            })?;
            let name_found = read_string(&self.storage, &wallet_keyspace, &string_key)?;

            self.delete_wallet_keyspace(&name_found)?;
        }
        self.storage.clear(wallet_keyspace).unwrap();
        self.storage.clear(KEYSPACE).unwrap();

        Ok(())
    }

    fn delete_wallet_keyspace(&self, name: &str) -> Result<()> {
        self.storage.delete(KEYSPACE, name)?;
        assert!(self.storage.get(KEYSPACE, name)?.is_none());
        let info_keyspace = get_info_keyspace(name);

        let stakingkeyset_keyspace = get_stakingkeyset_keyspace(name);
        let public_keyspace = get_public_keyspace(name);
        let private_keyspace = get_private_keyspace(name);
        let roothashset_keyspace = get_roothashset_keyspace(name);
        let multisigaddress_keyspace = get_multisig_keyspace(name);
        let wallet_keyspace = get_wallet_keyspace();
        self.storage.delete(wallet_keyspace, name)?;
        self.storage.clear(info_keyspace)?;
        self.storage.clear(roothashset_keyspace)?;

        self.storage.clear(stakingkeyset_keyspace)?;
        self.storage.clear(public_keyspace)?;
        self.storage.clear(private_keyspace)?;
        self.storage.clear(multisigaddress_keyspace)?;
        Ok(())
    }
    /// Delete the key
    pub fn delete(&self, name: &str, _enckey: &SecKey) -> Result<()> {
        self.storage.delete(KEYSPACE, name)?;
        self.delete_wallet_keyspace(name)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secstr::SecUtf8;

    use client_common::storage::MemoryStorage;
    use client_common::{seckey::derive_enckey, PrivateKey};

    #[test]
    fn check_flow() {
        let wallet_service = WalletService::new(MemoryStorage::default());

        let enckey = derive_enckey(&SecUtf8::from("passphrase"), "name").unwrap();

        let private_key = PrivateKey::new().unwrap();
        let view_key = PublicKey::from(&private_key);

        let error = wallet_service
            .public_keys("name", &enckey)
            .expect_err("Retrieved public keys for non-existent wallet");

        assert_eq!(error.kind(), ErrorKind::InvalidInput);

        assert!(wallet_service
            .create("name", &enckey, view_key.clone())
            .is_ok());

        let error = wallet_service
            .create("name", &enckey, view_key.clone())
            .expect_err("Created duplicate wallet");

        assert_eq!(error.kind(), ErrorKind::InvalidInput);

        assert_eq!(
            0,
            wallet_service.public_keys("name", &enckey).unwrap().len()
        );

        let error = wallet_service
            .create("name", &enckey, view_key)
            .expect_err("Able to create wallet with same name as previously created");

        assert_eq!(error.kind(), ErrorKind::InvalidInput, "Invalid error kind");

        let private_key = PrivateKey::new().unwrap();
        let public_key = PublicKey::from(&private_key);

        wallet_service
            .add_public_key("name", &enckey, &public_key)
            .unwrap();

        assert_eq!(
            1,
            wallet_service.public_keys("name", &enckey).unwrap().len()
        );

        wallet_service.clear().unwrap();

        let error = wallet_service
            .public_keys("name", &enckey)
            .expect_err("Retrieved public keys for non-existent wallet");

        assert_eq!(error.kind(), ErrorKind::InvalidInput);
    }
}
