use secstr::SecUtf8;
use zeroize::Zeroize;

use super::key_service_data::KeyServiceInterface;
use client_common::{PrivateKey, PublicKey, Result, SecureStorage, Storage};
const KEYSPACE: &str = "core_key";

/// Maintains mapping `public-key -> private-key`
#[derive(Debug, Default, Clone)]
pub struct BasicKeyService<T: Storage> {
    storage: T,
}

impl<T> KeyServiceInterface for BasicKeyService<T>
where
    T: Storage,
{
    /// Generates a new public-private keypair
    fn generate_keypair(
        &self,
        name: &str,
        passphrase: &SecUtf8,
        is_staking: bool,
    ) -> Result<(PublicKey, PrivateKey)> {
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

impl<T> BasicKeyService<T>
where
    T: Storage,
{
    /// Creates a new instance of key service
    pub fn new(storage: T) -> Self {
        BasicKeyService { storage }
    }
}
