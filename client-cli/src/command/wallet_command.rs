use client_common::{Error, ErrorKind, Result};
use client_core::WalletClient;
use quest::{ask, success};
use structopt::StructOpt;

use crate::ask_passphrase;
use client_core::types::WalletKind;
use secstr::*;
#[derive(Debug, StructOpt)]
pub enum WalletCommand {
    #[structopt(name = "new", about = "New wallet")]
    New {
        #[structopt(name = "name", short, long, help = "Name of wallet")]
        name: String,
    },
    #[structopt(name = "list", about = "List all wallets")]
    List,
}

impl WalletCommand {
    pub fn execute<T: WalletClient>(&self, wallet_client: T) -> Result<()> {
        match self {
            WalletCommand::New { name } => Self::new_wallet(wallet_client, name),
            WalletCommand::List => Self::list_wallets(wallet_client),
        }
    }

    fn get_mnemonics<T: WalletClient>(wallet_client: &T) -> SecUtf8 {
        let mut mnemonics: String;
        loop {
            println!("== hd wallet setup==");
            println!("1. create new mnemonics");
            println!("2. restore from mnemonics");
            println!("enter command=");

            let a = quest::text().unwrap();
            if a == "1" {
                mnemonics = wallet_client.new_mnemonics().unwrap().to_string();
            } else if a == "2" {
                println!("enter mnemonics=");
                mnemonics = quest::text().unwrap().to_string();
            } else {
                continue;
            }
            println!("mnemonics={}", mnemonics);
            println!("enter y to conitnue");
            let r = quest::yesno(false);
            if r.is_ok() && r.as_ref().unwrap().is_some() && r.as_ref().unwrap().unwrap() {
                break;
            }
        }
        SecUtf8::from(mnemonics)
    }
    fn ask_wallet_kind() -> WalletKind {
        loop {
            println!("== wallet choose==");
            println!("1. normal wallet");
            println!("2. hd wallet");
            println!("enter command=");

            let a = quest::text().unwrap();
            if a == "1" {
                return WalletKind::Basic;
            } else if a == "2" {
                return WalletKind::HD;
            } else {
                continue;
            }
        }
    }
    fn new_wallet<T: WalletClient>(wallet_client: T, name: &str) -> Result<()> {
        let passphrase = ask_passphrase(None)?;
        let confirmed_passphrase = ask_passphrase(Some("Confirm passphrase: "))?;

        if passphrase != confirmed_passphrase {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Passphrases do not match",
            ));
        }

        let walletkind = WalletCommand::ask_wallet_kind();
        if WalletKind::HD == walletkind {
            let mnemonics = WalletCommand::get_mnemonics(&wallet_client);
            println!("ok keep mnemonics safely={}", mnemonics.unsecure());
            wallet_client.new_hdwallet(name, &passphrase, &mnemonics)?;
        }
        println!("--------------------------------------------");
        wallet_client.new_wallet(name, &passphrase)?;
        success(&format!("Wallet created with name: {}", name));
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
