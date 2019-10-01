//! Management services
mod basic_key_service;
mod global_state_service;
mod hdkey_service;
mod key_service;
mod key_service_data;
mod multi_sig_session_service;
mod root_hash_service;
mod wallet_service;
mod wallet_state_service;

#[doc(hidden)]
pub use self::wallet_state_service::WalletStateMemento;

pub use self::global_state_service::GlobalStateService;
pub use self::key_service::KeyService;
pub use self::key_service_data::WalletKinds;
pub use self::multi_sig_session_service::MultiSigSessionService;
pub use self::root_hash_service::RootHashService;
pub use self::wallet_service::WalletService;
pub use self::wallet_state_service::WalletStateService;

// BASIC : normal wallet
// HD: hd wallet
/// get wallet kind from env
pub fn get_wallet_kind() -> WalletKinds {
    let walletkind = std::env::var("CRYPTO_WALLET_KIND")
        .map(Some)
        .unwrap_or(None);
    let r = if let Some(a) = walletkind {
        match a.as_str() {
            "HD" => WalletKinds::HD,
            _ => WalletKinds::Basic,
        }
    } else {
        WalletKinds::Basic
    };
    println!("founded wallet {:?}", r);
    r
}
