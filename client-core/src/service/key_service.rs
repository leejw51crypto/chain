use secstr::SecUtf8;
use zeroize::Zeroize;

use super::basic_key_service::BasicKeyService;
use super::hdkey_service::HDKeyService;
use super::key_service_data::{KeyServiceInterface, WalletKinds};
use client_common::{PrivateKey, PublicKey, Result, SecureStorage, Storage};
const KEYSPACE: &str = "core_key";

/// Maintains mapping `public-key -> private-key`
#[derive(Debug, Default, Clone)]
pub struct KeyService<T: Storage> {
    kind: WalletKinds,
    basic: Option<BasicKeyService<T>>,
    hd: Option<HDKeyService<T>>,
}

impl<T> KeyService<T>
where
    T: Storage,
{
    /// Creates a new instance of key service
    pub fn new(storage: T, stroage2: T, kind: WalletKinds) -> Self {
        KeyService {
            kind,
            basic: Some(BasicKeyService::new(storage)),
            hd: Some(HDKeyService::new(stroage2)),
        }
    }

    /// get random mnemonic
    pub fn get_random_mnemonic(&self) -> String {
        match self.kind {
            WalletKinds::Basic => String::new(),
            WalletKinds::HD => self.hd.as_ref().unwrap().get_random_mnemonic(),
        }
    }

    /// restore from mnemonic
    pub fn generate_seed(&self, mnemonic: &str, name: &str, passphrase: &SecUtf8) -> Result<()> {
        match self.kind {
            WalletKinds::Basic => Ok(()),
            WalletKinds::HD => self
                .hd
                .as_ref()
                .unwrap()
                .generate_seed(mnemonic, name, passphrase),
        }
    }
    /// Generates a new public-private keypair
    pub fn generate_keypair(
        &self,
        name: &str,
        passphrase: &SecUtf8,
        is_staking: bool,
    ) -> Result<(PublicKey, PrivateKey)> {
        match self.kind {
            WalletKinds::Basic => self
                .basic
                .as_ref()
                .unwrap()
                .generate_keypair(name, passphrase, is_staking),

            WalletKinds::HD => self
                .hd
                .as_ref()
                .unwrap()
                .generate_keypair(name, passphrase, is_staking),
        }
    }

    /// Retrieves private key corresponding to given public key
    pub fn private_key(
        &self,
        public_key: &PublicKey,
        passphrase: &SecUtf8,
    ) -> Result<Option<PrivateKey>> {
        match self.kind {
            WalletKinds::Basic => self
                .basic
                .as_ref()
                .unwrap()
                .private_key(public_key, passphrase),
            WalletKinds::HD => self
                .hd
                .as_ref()
                .unwrap()
                .private_key(public_key, passphrase),
        }
    }

    /// Clears all storage
    pub fn clear(&self) -> Result<()> {
        match self.kind {
            WalletKinds::Basic => self.basic.as_ref().unwrap().clear(),
            WalletKinds::HD => self.hd.as_ref().unwrap().clear(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use client_common::storage::MemoryStorage;
    use client_common::ErrorKind;

    #[test]
    fn check_flow() {
        let key_service = KeyService::new(
            MemoryStorage::default(),
            MemoryStorage::default(),
            WalletKinds::Basic,
        );
        let passphrase = SecUtf8::from("passphrase");

        let (public_key, private_key) = key_service
            .generate_keypair("", &passphrase, false)
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
