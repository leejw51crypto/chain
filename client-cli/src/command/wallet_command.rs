use quest::{ask, success};
use structopt::StructOpt;

use client_common::{Error, ErrorKind, Result};
use client_core::WalletClient;

use crate::ask_passphrase;
use super::super::command::get_wallet_kind;
use client_core::service::WalletKinds;
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


    fn get_mnemonics<T:WalletClient>(wallet_client:&T ) -> String {
           let mut mnemonics = "".to_string();
        loop {
            println!("== hd wallet setup==");
            println!("1. create new mnemonics");
            println!("2. restore from mnemonics");
            println!("enter command=");

            let a = quest::text().unwrap();
            if a == "1" {
                mnemonics = wallet_client.new_mnemonics().unwrap();
            } else if a == "2" {
                println!("enter mnemonics=");
                mnemonics = quest::text().unwrap().to_string();
            } else {
                continue;
            }
            println!("mnemonics={}", mnemonics);
            println!("enter y to conitnue");
            let r = quest::yesno(false);
            if r.is_ok() {
                if r.as_ref().unwrap().is_some() {
                    if r.as_ref().unwrap().unwrap() {
                        break;
                    }
                }
            }
        }
        mnemonics
    }

    fn new_wallet<T: WalletClient>(wallet_client: T, name: &str) -> Result<()> {
        let passphrase = ask_passphrase(None)?;
        let confirmed_passphrase = ask_passphrase(Some("Confirm passphrase: "))?;
        let mut mnemonics = "".to_string();

        if passphrase != confirmed_passphrase {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Passphrases do not match",
            ));
        }

        let walletkind= get_wallet_kind();
        match walletkind {
            WalletKinds::HD => {
                mnemonics= WalletCommand::get_mnemonics(&wallet_client);
        println!("ok keep mnemonics safely={}", mnemonics);
        wallet_client.new_hdwallet(name, &passphrase, mnemonics)?;
        
            },
            _ => {

            }
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
