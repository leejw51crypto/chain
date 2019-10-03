use quest::{ask, success};
use structopt::StructOpt;

use client_common::{Error, ErrorKind, Result};
use client_core::WalletClient;

use crate::ask_passphrase;
use client_core::types::WalletKind;
#[derive(Debug, StructOpt)]
pub enum WalletCommand {
    #[structopt(name = "new", about = "New wallet")]
    New {
        #[structopt(name = "name", short, long, help = "Name of wallet")]
        name: String,
        #[structopt(
            name = "type",
            short,
            long,
            help = "Type of wallet to create (hd, basic)"
        )]
        wallet_type: WalletKind,
    },
    #[structopt(name = "list", about = "List all wallets")]
    List,
    #[structopt(name = "restore", about = "Restore HD Wallet")]
    Restore {
        #[structopt(name = "name", short, long, help = "Name of wallet")]
        name: String,
        #[structopt(
            name = "mnemonic",
            short,
            long,
            help = "Mnemonic of wallet (bip39 compatible mnemonics), such as \"word1 word2 ... \""
        )]
        mnemonic: String,
    },
}

impl WalletCommand {
    pub fn execute<T: WalletClient>(&self, wallet_client: T) -> Result<()> {
        match self {
            WalletCommand::New { name, wallet_type } => {
                Self::new_wallet(wallet_client, name, *wallet_type)
            }
            WalletCommand::List => Self::list_wallets(wallet_client),
            WalletCommand::Restore { name, mnemonic } => {
                Self::restore_wallet(wallet_client, name, mnemonic)
            }
        }
    }

    fn new_wallet<T: WalletClient>(
        wallet_client: T,
        name: &str,
        walletkind: WalletKind,
    ) -> Result<()> {
        let passphrase = ask_passphrase(None)?;
        let confirmed_passphrase = ask_passphrase(Some("Confirm passphrase: "))?;

        if passphrase != confirmed_passphrase {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Passphrases do not match",
            ));
        }

        if WalletKind::HD == walletkind {
            let mnemonic = wallet_client.new_mnemonics().unwrap();
            println!("ok keep mnemonics safely={}", mnemonic);
            wallet_client.new_hdwallet(name, &passphrase, mnemonic)?;
        }
        println!("--------------------------------------------");
        wallet_client.new_wallet(name, &passphrase)?;
        success(&format!("Wallet created with name: {}", name));
        Ok(())
    }

    fn restore_wallet<T: WalletClient>(wallet_client: T, name: &str, mnemonic: &str) -> Result<()> {
        let passphrase = ask_passphrase(None)?;
        let confirmed_passphrase = ask_passphrase(Some("Confirm passphrase: "))?;

        if passphrase != confirmed_passphrase {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Passphrases do not match",
            ));
        }

        println!("ok keep mnemonics safely={}", mnemonic);
        wallet_client.new_hdwallet(name, &passphrase, mnemonic.to_string())?;
        println!("--------------------------------------------");
        wallet_client.new_wallet(name, &passphrase)?;
        success(&format!("Wallet restore with name: {}", name));
        Ok(())
    }

    fn list_wallets<T: WalletClient>(wallet_client: T) -> Result<()> {
        let wallets = wallet_client.wallets()?;

        if !wallets.is_empty() {
            for wallet in wallets {
                ask("Wallet name: ");
                success(&wallet);
            }
        } else {
            success("No wallets found!")
        }

        Ok(())
    }
}
