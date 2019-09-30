use secstr::SecUtf8;
use zeroize::Zeroize;

use client_common::{PrivateKey, PublicKey, Result, SecureStorage, Storage};

/// key service interface
pub trait KeyServiceInterface {
    /// Generates a new public-private keypair
    fn generate_keypair(
        &self,
        name: &str,
        passphrase: &SecUtf8,
        is_staking: bool,
    ) -> Result<(PublicKey, PrivateKey)>;
    /// Retrieves private key corresponding to given public key
    fn private_key(
        &self,
        public_key: &PublicKey,
        passphrase: &SecUtf8,
    ) -> Result<Option<PrivateKey>>;
    /// Clears all storage
    fn clear(&self) -> Result<()>;
}
