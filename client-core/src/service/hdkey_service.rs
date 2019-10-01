use bip39::{Language, Mnemonic, MnemonicType, Seed};
use secstr::SecUtf8;
use zeroize::Zeroize;

use client_common::{PrivateKey, PublicKey, Result, SecureStorage, Storage};
use log::debug;
use tiny_hderive::bip32::ExtendedPrivKey;
const KEYSPACE: &str = "core_hdkey";
use super::key_service_data::KeyServiceInterface;

/// Maintains mapping `public-key -> private-key`
#[derive(Debug, Default, Clone)]
pub struct HDKeyService<T: Storage> {
    storage: T,
}

impl<T> KeyServiceInterface for HDKeyService<T>
where
    T: Storage,
{
    /// m / purpose' / coin_type' / account' / change / address_index
    /// account: donation, savings, common expense
    /// change: 0: external, 1: internal
    /// Generates a new public-private keypair
    fn generate_keypair(
        &self,
        name: &str,           // wallet name
        passphrase: &SecUtf8, // wallet pass phrase
        is_staking: bool,     // kind of address
    ) -> Result<(PublicKey, PrivateKey)> {
        let seed_bytes = self.storage.get_secure(KEYSPACE, name, passphrase)?;
        let mut index = if is_staking {
            self.read_number(passphrase, format!("staking_{}", name).as_bytes(), 0)
        } else {
            self.read_number(passphrase, format!("transfer_{}", name).as_bytes(), 0)
        };
        debug!("hdwallet index={}", index);
        let account = if is_staking { 1 } else { 0 };
        let extended = ExtendedPrivKey::derive(
            &seed_bytes.clone().unwrap(),
            format!("m/44'/394'/{}'/0/{}", account, index).as_str(),
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

    /// Retrieves private key corresponding to given public key
    fn private_key(
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
    fn clear(&self) -> Result<()> {
        self.storage.clear(KEYSPACE)
    }
}

impl<T> HDKeyService<T>
where
    T: Storage,
{
    /// get random mnemonic
    pub fn get_random_mnemonic(&self) -> String {
        let mnemonic = Mnemonic::new(MnemonicType::Words24, Language::English);
        mnemonic.clone().to_string()
    }

    /// generate seed from mnemonic
    pub fn generate_seed(&self, mnemonic: &str, name: &str, passphrase: &SecUtf8) -> Result<()> {
        debug!("hdwallet generate seed={}", mnemonic);
        let mnemonic = Mnemonic::from_phrase(&mnemonic.to_string(), Language::English).unwrap();
        let seed = Seed::new(&mnemonic, "");
        self.storage
            .set_secure(KEYSPACE, name, seed.as_bytes().into(), passphrase)?;
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
        for index in 0..count {
            let seed_bytes = self.storage.get_secure(KEYSPACE, name, passphrase)?;
            for account in 0..2 {
                let extended = ExtendedPrivKey::derive(
                    &seed_bytes.clone().unwrap()[..],
                    format!("m/44'/394'/{}'/0/{}", account, index).as_str(),
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

    /// Creates a new instance of key service
    pub fn new(storage: T) -> Self {
        HDKeyService { storage }
    }

    /// read value from db
    #[allow(dead_code)]
    pub fn read_value(&self, passphrase: &SecUtf8, key: &[u8], default: &[u8]) -> Vec<u8> {
        if let Ok(connected) = self.storage.get_secure(KEYSPACE, key, passphrase) {
            if let Some(value) = connected {
                return value.clone();
            }
        }
        default.to_vec()
    }

    /// read number
    pub fn read_number(&self, passphrase: &SecUtf8, key: &[u8], default: u32) -> u32 {
        if let Ok(connected) = self.storage.get_secure(KEYSPACE, key, passphrase) {
            if let Some(value) = connected {
                return std::str::from_utf8(&value[..])
                    .unwrap()
                    .parse::<u32>()
                    .unwrap();
            }
        }
        default
    }

    /// write number
    pub fn write_number(&self, passphrase: &SecUtf8, key: &[u8], value: u32) {
        let a = value.to_string();
        let b = a.as_bytes();
        self.storage
            .set_secure(KEYSPACE, key, b.to_vec(), passphrase)
            .unwrap();
    }
}
