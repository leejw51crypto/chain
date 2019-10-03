use secstr::SecUtf8;
use zeroize::Zeroize;

use bip39::{Language, Mnemonic, MnemonicType, Seed};

use client_common::{PrivateKey, PublicKey, Result, SecureStorage, Storage};
const KEYSPACE: &str = "core_key";
const KEYSPACE_HD: &str = "hd_key";
use chain_core::init::network::get_bip44_coin_type;
use log::debug;
use tiny_hderive::bip32::ExtendedPrivKey;

/// Maintains mapping `public-key -> private-key`
#[derive(Debug, Default, Clone)]
pub struct KeyService<T: Storage> {
    storage: T,
}

impl<T> KeyService<T>
where
    T: Storage,
{
    /// Creates a new instance of key service
    pub fn new(storage: T) -> Self {
        KeyService { storage }
    }

    /// Generates keypair by wallet kinds recorded in sled storage
    pub fn generate_keypair_auto(
        &self,
        name: &str,
        passphrase: &SecUtf8,
        is_staking: bool,
    ) -> Result<(PublicKey, PrivateKey)> {
        if self.is_hd_wallet(name, passphrase) {
            self.generate_keypair_hd(name, passphrase, is_staking)
        } else {
            self.generate_keypair_basic(passphrase)
        }
    }

    /// Generates a new public-private keypair
    pub fn generate_keypair_basic(&self, passphrase: &SecUtf8) -> Result<(PublicKey, PrivateKey)> {
        let private_key = PrivateKey::new()?;
        let public_key = PublicKey::from(&private_key);

        self.storage.set_secure(
            KEYSPACE,
            public_key.serialize(),
            private_key.serialize(),
            passphrase,
        )?;

        Ok((public_key, private_key))
    }

    /// Retrieves private key corresponding to given public key
    pub fn private_key(
        &self,
        public_key: &PublicKey,
        passphrase: &SecUtf8,
    ) -> Result<Option<PrivateKey>> {
        let private_key_bytes =
            self.storage
                .get_secure(KEYSPACE, public_key.serialize(), passphrase)?;

        private_key_bytes
            .map(|mut private_key_bytes| {
                let private_key = PrivateKey::deserialize_from(&private_key_bytes)?;
                private_key_bytes.zeroize();
                Ok(private_key)
            })
            .transpose()
    }

    /// Clears all storage
    pub fn clear(&self) -> Result<()> {
        self.storage.clear(KEYSPACE).expect("clear keyspace");
        self.storage
            .clear(KEYSPACE_HD)
            .expect("clear keyspace for hd");
        Ok(())
    }

    /// get random mnemonic
    pub fn get_random_mnemonic(&self) -> String {
        let mnemonic = Mnemonic::new(MnemonicType::Words24, Language::English);
        mnemonic.to_string()
    }

    /// is hd
    pub fn is_hd_wallet(&self, name: &str, passphrase: &SecUtf8) -> bool {
        let key = name.as_bytes();
        let value = self.read_value(passphrase, &key[..]);
        value.is_some()
    }
    /// generate seed from mnemonic
    pub fn generate_seed(&self, mnemonic: &str, name: &str, passphrase: &SecUtf8) -> Result<()> {
        debug!("hdwallet generate seed={}", mnemonic);
        let mnemonic = Mnemonic::from_phrase(&mnemonic.to_string(), Language::English).unwrap();
        let seed = Seed::new(&mnemonic, "");
        self.storage
            .set_secure(KEYSPACE_HD, name, seed.as_bytes().into(), passphrase)?;
        debug!("hdwallet write seed success");
        Ok(())
    }

    /// auto-matically generate staking, transfer addresses
    /// with just one api call
    #[allow(dead_code)]
    pub fn auto_restore(
        &self,
        mnemonic: &str,
        name: &str,
        passphrase: &SecUtf8,
        count: i32,
    ) -> Result<()> {
        self.generate_seed(mnemonic, name, passphrase)
            .expect("auto restore");
        let cointype = get_bip44_coin_type();
        println!("coin type={}", cointype);
        for index in 0..count {
            let seed_bytes = self.storage.get_secure(KEYSPACE_HD, name, passphrase)?;
            for account in 0..2 {
                let extended = ExtendedPrivKey::derive(
                    &seed_bytes.clone().unwrap()[..],
                    format!("m/44'/{}'/{}'/0/{}", cointype, account, index).as_str(),
                )
                .unwrap();
                let secret_key_bytes = extended.secret();

                let private_key = PrivateKey::deserialize_from(&secret_key_bytes).unwrap();
                let public_key = PublicKey::from(&private_key);

                self.storage.set_secure(
                    KEYSPACE,
                    public_key.serialize(),
                    private_key.serialize(),
                    passphrase,
                )?;
            }
        }
        Ok(())
    }

    /// read value from db, if it's None, there value doesn't exist
    pub fn read_value(&self, passphrase: &SecUtf8, key: &[u8]) -> Option<Vec<u8>> {
        if let Ok(connected) = self.storage.get_secure(KEYSPACE_HD, key, passphrase) {
            if let Some(value) = connected {
                return Some(value.clone());
            }
        }
        None
    }

    /// read number, if value doesn't exist, it returns default value
    pub fn read_number(&self, passphrase: &SecUtf8, key: &[u8], default: u32) -> u32 {
        if let Ok(connected) = self.storage.get_secure(KEYSPACE_HD, key, passphrase) {
            if let Some(value) = connected {
                return std::str::from_utf8(&value[..])
                    .unwrap()
                    .parse::<u32>()
                    .unwrap();
            }
        }
        default
    }

    /// write number to store, write number as string
    /// writes hdwallet index, after making a new entry, index increases by 1
    /// so address is generated in deterministic way.
    pub fn write_number(&self, passphrase: &SecUtf8, key: &[u8], value: u32) {
        let a = value.to_string();
        let b = a.as_bytes();
        self.storage
            .set_secure(KEYSPACE_HD, key, b.to_vec(), passphrase)
            .unwrap();
    }

    /// m / purpose' / coin_type' / account' / change / address_index
    /// account: donation, savings, common expense
    /// change: 0: external, 1: internal
    /// Generates a new public-private keypair
    pub fn generate_keypair_hd(
        &self,
        name: &str,           // wallet name
        passphrase: &SecUtf8, // wallet pass phrase
        is_staking: bool,     // kind of address
    ) -> Result<(PublicKey, PrivateKey)> {
        let seed_bytes = self.storage.get_secure(KEYSPACE_HD, name, passphrase)?;
        let mut index = if is_staking {
            self.read_number(passphrase, format!("staking_{}", name).as_bytes(), 0)
        } else {
            self.read_number(passphrase, format!("transfer_{}", name).as_bytes(), 0)
        };
        debug!("hdwallet index={}", index);
        let cointype = get_bip44_coin_type();
        println!("coin type={}", cointype);
        let account = if is_staking { 1 } else { 0 };
        let extended = ExtendedPrivKey::derive(
            &seed_bytes.unwrap(),
            format!("m/44'/{}'/{}'/0/{}", cointype, account, index).as_str(),
        )
        .unwrap();
        let secret_key_bytes = extended.secret();
        debug!("hdwallet save index={}", index);
        let private_key = PrivateKey::deserialize_from(&secret_key_bytes).unwrap();
        let public_key = PublicKey::from(&private_key);
        self.storage.set_secure(
            KEYSPACE,
            public_key.serialize(),
            private_key.serialize(),
            passphrase,
        )?;
        // done
        index += 1;
        if is_staking {
            self.write_number(passphrase, format!("staking_{}", name).as_bytes(), index);
        } else {
            self.write_number(passphrase, format!("transfer_{}", name).as_bytes(), index);
        }

        Ok((public_key, private_key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use client_common::storage::MemoryStorage;
    use client_common::ErrorKind;

    #[test]
    fn check_flow() {
        let key_service = KeyService::new(MemoryStorage::default());
        let passphrase = SecUtf8::from("passphrase");

        let (public_key, private_key) = key_service
            .generate_keypair_basic(&passphrase)
            .expect("Unable to generate private key");

        let retrieved_private_key = key_service
            .private_key(&public_key, &passphrase)
            .unwrap()
            .unwrap();

        assert_eq!(private_key, retrieved_private_key);

        let error = key_service
            .private_key(&public_key, &SecUtf8::from("incorrect_passphrase"))
            .expect_err("Decryption worked with incorrect passphrase");

        assert_eq!(
            error.kind(),
            ErrorKind::DecryptionError,
            "Invalid error kind"
        );

        assert!(key_service.clear().is_ok());
    }
}
