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
#[derive(Debug)]
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
        (self.key_pairs.len() as u64).encode_to(dest);
        for (key, value) in self.key_pairs.iter() {
            key.encode_to(dest);
            value.encode_to(dest);
        }
        (self.public_keys.len() as u64).encode_to(dest);
        for key in self.public_keys.iter() {
            key.encode_to(dest);
        }
        (self.staking_keys.len() as u64).encode_to(dest);
        for key in self.staking_keys.iter() {
            key.encode_to(dest);
        }
        (self.root_hashes.len() as u64).encode_to(dest);
        for hash in self.root_hashes.iter() {
            hash.encode_to(dest);
        }
    }
}

impl Decode for Wallet {
    fn decode<I: Input>(input: &mut I) -> std::result::Result<Self, parity_scale_codec::Error> {
        let view_key = PublicKey::decode(input)?;
        let mut key_pairs = BTreeMap::new();
        let len = u64::decode(input)?;
        for _ in 0..len {
            let key = PublicKey::decode(input)?;
            let value = PrivateKey::decode(input)?;
            key_pairs.insert(key, value);
        }
        let len = u64::decode(input)?;
        let mut public_keys = IndexSet::with_capacity(len as usize);
        for _ in 0..len {
            public_keys.insert(PublicKey::decode(input)?);
        }

        let len = u64::decode(input)?;
        let mut staking_keys = IndexSet::with_capacity(len as usize);
        for _ in 0..len {
            staking_keys.insert(PublicKey::decode(input)?);
        }

        let len = u64::decode(input)?;
        let mut root_hashes = IndexSet::with_capacity(len as usize);
        for _ in 0..len {
            root_hashes.insert(H256::decode(input)?);
        }

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

/// Load wallet from storage
pub fn load_wallet<S: SecureStorage>(
    storage: &S,
    name: &str,
    enckey: &SecKey,
) -> Result<Option<Wallet>> {
    storage.load_secure(KEYSPACE, name, enckey)
}

/// Save wallet to storage
pub fn save_wallet<S: SecureStorage>(
    storage: &S,
    name: &str,
    enckey: &SecKey,
    wallet: &Wallet,
) -> Result<()> {
    storage.save_secure(KEYSPACE, name, enckey, wallet)
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
    pub fn get_wallet_state(&self, name: &str, enckey: &SecKey) -> Result<WalletState> {
        load_wallet_state(&self.storage, name, enckey)?.err_kind(ErrorKind::InvalidInput, || {
            format!("WalletState with name ({}) not found", name)
        })
    }

    /// Store the wallet to storage
    pub fn set_wallet(&self, name: &str, enckey: &SecKey, wallet: Wallet) -> Result<()> {
        save_wallet(&self.storage, name, enckey, &wallet)
    }

    /// Finds staking key corresponding to given redeem address
    pub fn find_staking_key(
        &self,
        name: &str,
        enckey: &SecKey,
        redeem_address: &RedeemAddress,
    ) -> Result<Option<PublicKey>> {
        let stakingkeyset_keyspace = format!("{}_{}_stakingkey_set", KEYSPACE, name);

        let value = self.storage.get(
            stakingkeyset_keyspace,
            redeem_address.to_string().as_bytes(),
        )?;
        if let Some(raw_value) = value {
            let pubkey = PublicKey::deserialize_from(&raw_value)?;
            return Ok(Some(pubkey));
        }

        return Err(Error::new(ErrorKind::InvalidInput, "staking_key not found"));
    }

    /// Finds private_key corresponding to given public_key
    pub fn find_private_key(
        &self,
        name: &str,
        enckey: &SecKey,
        public_key: &PublicKey,
    ) -> Result<Option<PrivateKey>> {
        let private_keyspace = format!("{}_{}_privatekey", KEYSPACE, name);

        // key: public_key
        // value: private_key
        let value =
            self.storage
                .get_secure(private_keyspace.clone(), public_key.serialize(), enckey)?;
        if let Some(raw_value) = value {
            let privatekey = PrivateKey::deserialize_from(&raw_value)?;
            return Ok(Some(privatekey));
        }

        return Err(Error::new(ErrorKind::InvalidInput, "private_key not found"));
    }

    /// Checks if root hash exists in current wallet and returns root hash if exists
    pub fn find_root_hash(
        &self,
        name: &str,
        enckey: &SecKey,
        address: &ExtendedAddr,
    ) -> Result<Option<H256>> {
        /*match address {
            ExtendedAddr::OrTree(ref root_hash) => {
        //        self.root_hashes.iter().find(|hash| hash == &root_hash)
            }
        }*/
        if let ExtendedAddr::OrTree(ref root_hash) = address {
            // roothashset
            let roothashset_keyspace = format!("{}_{}_roothash_set", KEYSPACE, name);
            let value = self.storage.get(roothashset_keyspace, root_hash.to_vec())?;
            if let Some(raw_value) = value {
                let mut roothash_found: H256 = H256::default();
                roothash_found.copy_from_slice(&raw_value);
                return Ok(Some(roothash_found));
            }
        }

        return Err(Error::new(ErrorKind::InvalidInput, "private_key not found"));
    }

    /// Creates a new wallet and returns wallet ID
    pub fn create(&self, name: &str, enckey: &SecKey, view_key: PublicKey) -> Result<()> {
        if self.storage.contains_key(KEYSPACE, name)? {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Wallet with name ({}) already exists", name),
            ));
        }

        self.set_wallet(name, enckey, Wallet::new(view_key));

        let mut index_value: u64 = self.read_number(KEYSPACE, "walletindex")?;

        // key: index
        // value: walletname
        let wallet_keyspace = format!("{}_walletname", KEYSPACE);
        self.storage.set(
            wallet_keyspace.clone(),
            format!("{}", index_value),
            name.as_bytes().to_vec(),
        )?;

        assert!(
            self.read_string(&wallet_keyspace, &format!("{}", index_value))
                .unwrap()
                == name
        );
        println!("create wallet {} {}", index_value, name);

        // increase
        index_value = index_value + 1;
        println!("write index value {}", index_value);
        self.write_number(KEYSPACE, "walletindex", index_value)?;

        assert!(self.read_number(KEYSPACE, "walletindex")? == index_value);
        println!("OK");
        Ok(())
    }

    /// Returns view key of wallet
    pub fn view_key(&self, name: &str, enckey: &SecKey) -> Result<PublicKey> {
        let wallet = self.get_wallet(name, enckey)?;
        Ok(wallet.view_key)
    }

    /// Returns all public keys stored in a wallet
    pub fn public_keys(&self, name: &str, enckey: &SecKey) -> Result<IndexSet<PublicKey>> {
        let info_keyspace = format!("{}_{}_info", KEYSPACE, name);
        let mut publickey_count: u64 = self.read_number(&info_keyspace, "publickeyindex")?;

        let public_keyspace = format!("{}_{}_publickey", KEYSPACE, name);
        let mut ret: IndexSet<PublicKey> = IndexSet::<PublicKey>::new();
        for i in 0..publickey_count {
            let value = self.storage.get(&public_keyspace, format!("{}", i))?;
            if let Some(raw_value) = value {
                let pubkey = PublicKey::deserialize_from(&raw_value)?;
                ret.insert(pubkey);
            }
        }
        assert!(publickey_count == ret.len() as u64);
        Ok(ret)
        //let wallet = self.get_wallet(name, enckey)?;
        //Ok(wallet.public_keys)
    }

    /// Returns all public keys corresponding to staking addresses stored in a wallet
    pub fn staking_keys(&self, name: &str, enckey: &SecKey) -> Result<IndexSet<PublicKey>> {
        let wallet = self.get_wallet(name, enckey)?;
        Ok(wallet.staking_keys)
    }

    /// Returns all multi-sig addresses stored in a wallet
    pub fn root_hashes(&self, name: &str, enckey: &SecKey) -> Result<IndexSet<H256>> {
        let wallet = self.get_wallet(name, enckey)?;
        Ok(wallet.root_hashes)
    }

    /// Returns all staking addresses stored in a wallet
    pub fn staking_addresses(
        &self,
        name: &str,
        enckey: &SecKey,
    ) -> Result<IndexSet<StakedStateAddress>> {
        Ok(self.get_wallet(name, enckey)?.staking_addresses())
    }

    /// Returns all tree addresses stored in a wallet
    pub fn transfer_addresses(
        &self,
        name: &str,
        enckey: &SecKey,
    ) -> Result<IndexSet<ExtendedAddr>> {
        Ok(self.get_wallet(name, enckey)?.transfer_addresses())
    }

    /// Adds a (public_key, private_key) pair to given wallet
    pub fn add_key_pairs(
        &self,
        name: &str,
        enckey: &SecKey,
        public_key: &PublicKey,
        private_key: &PrivateKey,
    ) -> Result<()> {
        let private_keyspace = format!("{}_{}_privatekey", KEYSPACE, name);

        // key: public_key
        // value: private_key
        self.storage.set_secure(
            private_keyspace.clone(),
            public_key.serialize(),
            private_key.serialize(),
            enckey,
        )?;
        if let Ok(value) =
            self.storage
                .get_secure(private_keyspace.clone(), public_key.serialize(), enckey)
        {
            if let Some(raw_value) = value {
                println!("private= {}", hex::encode(raw_value));
            }
        }

        Ok(())

        /*
        self.modify_wallet(name, enckey, move |wallet| {
            wallet
                .key_pairs
                .insert(public_key.clone(), private_key.clone());
        })*/
    }

    /// Adds a public key to given wallet
    pub fn add_public_key(
        &self,
        name: &str,
        enckey: &SecKey,
        public_key: &PublicKey,
    ) -> Result<()> {
        /* self.modify_wallet(name, enckey, move |wallet| {
            wallet.public_keys.insert(public_key.clone());
        })*/

        let info_keyspace = format!("{}_{}_info", KEYSPACE, name);
        let mut index_value: u64 = self.read_number(&info_keyspace, "publickeyindex")?;

        // key: index
        // value: publickey
        let public_keyspace = format!("{}_{}_publickey", KEYSPACE, name);
        self.storage.set(
            public_keyspace,
            format!("{}", index_value),
            public_key.serialize(),
        )?;
        println!("{} {}", index_value, hex::encode(&public_key.serialize()));

        // increase
        index_value = index_value + 1;
        self.write_number(&info_keyspace, "publickeyindex", index_value)?;

        Ok(())
    }

    /// Adds a public key corresponding to a staking address to given wallet
    pub fn add_staking_key(
        &self,
        name: &str,
        enckey: &SecKey,
        staking_key: &PublicKey,
    ) -> Result<()> {
        let info_keyspace = format!("{}_{}_info", KEYSPACE, name);
        let mut index_value: u64 = self.read_number(&info_keyspace, "stakingkeyindex")?;

        // key: index
        // value: stakingkey
        let stakingkey_keyspace = format!("{}_{}_stakingkey", KEYSPACE, name);
        self.storage.set(
            stakingkey_keyspace,
            format!("{}", index_value),
            staking_key.serialize(),
        )?;

        // stakingkey set
        // key: redeem address (20 bytes)
        // value: staking key (<-publickey)
        let redeemaddress = RedeemAddress::from(staking_key).to_string();
        let stakingkeyset_keyspace = format!("{}_{}_stakingkey_set", KEYSPACE, name);
        self.storage.set(
            stakingkeyset_keyspace,
            redeemaddress.as_bytes(),
            staking_key.serialize(),
        )?;

        // increase
        index_value = index_value + 1;
        self.write_number(&info_keyspace, "stakingkeyindex", index_value)?;

        Ok(())
    }

    /*
    fn modify_wallet<F>(&self, name: &str, enckey: &SecKey, f: F) -> Result<()>
    where
        F: Fn(&mut Wallet),
    {
        assert!(false);
        self.storage
            .fetch_and_update_secure(KEYSPACE, name, enckey, move |value| {
                let mut wallet_bytes = value.chain(|| {
                    (
                        ErrorKind::InvalidInput,
                        format!("Wallet with name ({}) not found", name),
                    )
                })?;
                let mut wallet = Wallet::decode(&mut wallet_bytes).chain(|| {
                    (
                        ErrorKind::DeserializationError,
                        format!("Unable to deserialize wallet with name {}", name),
                    )
                })?;
                f(&mut wallet);
                Ok(Some(wallet.encode()))
            })
            .map(|_| ())
    }*/

    fn read_number(&self, keyspace: &str, key: &str) -> Result<u64> {
        let value = self.storage.get(keyspace, key.as_bytes())?;
        if let Some(raw_value) = value {
            let mut v: [u8; 8] = [0; 8];
            v.copy_from_slice(&raw_value);
            let index_value: u64 = u64::from_be_bytes(v);
            return Ok(index_value);
        }
        Ok(0)
    }

    fn read_string(&self, keyspace: &str, key: &str) -> Result<String> {
        let value = self.storage.get(keyspace, key.as_bytes())?;
        if let Some(raw_value) = value {
            let ret = str::from_utf8(&raw_value).unwrap();
            println!("read_string {} {}", key, ret);
            return Ok(ret.to_string());
        }
        Ok("".to_string())
    }

    fn write_number(&self, keyspace: &str, key: &str, value: u64) -> Result<()> {
        self.storage
            .set(
                keyspace.clone(),
                key.as_bytes(),
                value.to_be_bytes().to_vec(),
            )
            .expect("write storage");
        Ok(())
    }

    /// Adds a multi-sig address to given wallet
    pub fn add_root_hash(&self, name: &str, enckey: &SecKey, root_hash: H256) -> Result<()> {
        let info_keyspace = format!("{}_{}_info", KEYSPACE, name);
        let mut index_value: u64 = self.read_number(&info_keyspace, "roothashindex")?;

        // key: index
        // value: roothash
        let roothash_keyspace = format!("{}_{}_roothash", KEYSPACE, name);
        self.storage.set_secure(
            roothash_keyspace,
            format!("{}", index_value),
            root_hash.to_vec(),
            enckey,
        )?;
        println!("{} {}", index_value, hex::encode(&root_hash));

        // roothashset
        let roothashset_keyspace = format!("{}_{}_roothash_set", KEYSPACE, name);
        self.storage.set(
            roothashset_keyspace,
            root_hash.to_vec(),
            name.as_bytes().to_vec(),
        )?;

        // increase
        index_value = index_value + 1;
        self.write_number(&info_keyspace, "roothashindex", index_value)?;
        Ok(())
    }

    /// Retrieves names of all the stored wallets
    pub fn names(&self) -> Result<Vec<String>> {
        let keys = self.storage.keys(KEYSPACE)?;

        keys.into_iter()
            .map(|bytes| {
                String::from_utf8(bytes).chain(|| {
                    (
                        ErrorKind::DeserializationError,
                        "Unable to deserialize wallet names in storage",
                    )
                })
            })
            .collect()
    }

    /// Clears all storage
    pub fn clear(&self) -> Result<()> {
        let mut index_value: u64 = self.read_number(KEYSPACE, "walletindex")?;
        println!("wallet number {}", index_value);

        let wallet_keyspace = format!("{}_walletname", KEYSPACE);
        for i in 0..index_value {
            let wallet_name = self
                .read_string(&wallet_keyspace, &format!("{}", i))
                .unwrap();
            println!("processing {} = {}", i, wallet_name); //  let self.storage.get( wallet_keyspace,
                                                            //    format!("{}", index_value)).unwrap();
            self.delete_wallet_keyspace(&wallet_name).unwrap();
        }
        self.write_number(KEYSPACE, "wallet_index", 0)?;
        self.storage.clear(KEYSPACE).unwrap();
        println!("remove all  {}", KEYSPACE);
        Ok(())
    }

    fn delete_wallet_keyspace(&self, name: &str) -> Result<()> {
        self.storage.delete(KEYSPACE, name)?;
        assert!(self.storage.get(KEYSPACE, name)?.is_none());
        let info_keyspace = format!("{}_{}_info", KEYSPACE, name);
        let stakingkey_keyspace = format!("{}_{}_stakingkey", KEYSPACE, name);
        let stakingkeyset_keyspace = format!("{}_{}_stakingkey_set", KEYSPACE, name);
        let public_keyspace = format!("{}_{}_publickey", KEYSPACE, name);
        let private_keyspace = format!("{}_{}_privatekey", KEYSPACE, name);
        let roothash_keyspace = format!("{}_{}_roothash", KEYSPACE, name);
        let roothashset_keyspace = format!("{}_{}_roothash_set", KEYSPACE, name);
        let multisigaddress_keyspace = format!("core_wallet_{}_multisigaddress", name);
        self.storage.clear(info_keyspace)?;
        self.storage.clear(roothash_keyspace)?;
        self.storage.clear(roothashset_keyspace)?;
        self.storage.clear(stakingkey_keyspace)?;
        self.storage.clear(stakingkeyset_keyspace)?;
        self.storage.clear(public_keyspace)?;
        self.storage.clear(private_keyspace)?;
        self.storage.clear(multisigaddress_keyspace)?;
        Ok(())
    }
    /// Delete the key
    pub fn delete(&self, name: &str, enckey: &SecKey) -> Result<Wallet> {
        println!("delete wallet {}", name);
        let wallet = self.get_wallet(name, enckey)?;
        self.storage.delete(KEYSPACE, name)?;
        self.delete_wallet_keyspace(name)?;
        Ok(wallet)
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
