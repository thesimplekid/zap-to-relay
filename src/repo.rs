use crate::config::Settings;
use crate::db::Account;
use crate::db::Db;
use crate::error::Error;
use crate::nauthz_grpc::Event;
use crate::nostr::Nostr;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use tracing::debug;

use lightning_invoice::Invoice;
use nostr_sdk::prelude::*;

#[derive(Clone)]
pub struct Repo {
    db: Arc<Mutex<Db>>,
    client: Nostr
}


impl Repo {
    pub fn new(nos_client: Nostr) -> Self {
        Repo {
            db: Arc::new(Mutex::new(Db::new())),
            client: nos_client,
        }
    }

    pub fn add_account(&self, account: &Account) -> Result<(), Error> {
        self.db.lock().unwrap().write_account(account)
    }

    pub fn get_account(&self, pubkey: &str) -> Result<Option<Account>, Error> {
        self.db.lock().unwrap().read_account(pubkey)
    }

    pub async fn update_account(&self, pubkey: &str, amount: i64) -> Result<Account, Error> {
        let account = self.get_account(pubkey)?;

        let updated_account;

        if let Some(account) = account {
            updated_account = Account {
                pubkey: pubkey.to_string(),
                balance: (account.balance as i64 + amount) as u64,
            };

            self.add_account(&updated_account)?;
            // TODO: send balance update
        } else {
            updated_account = Account {
                pubkey: pubkey.to_string(),
                balance: amount as u64,
            };

            self.add_account(&updated_account)?;
            // TODO: Send admission DM
            self.client.send_admission_dm(pubkey).await?;
        }

        Ok(updated_account)
    }

    pub fn get_user_zaps(&self, pubkey: &str) -> Result<HashSet<String>, Error> {
        self.db.lock().unwrap().read_zap(pubkey)
    }

    pub fn add_zap(&self, pubkey: &str, zap_id: &str) -> Result<(), Error> {
        self.db.lock().unwrap().write_zap(pubkey, zap_id)
    }

    pub async fn handle_zap(&self, event: Event) -> Result<()> {
        let (pubkey, amount) = read_zap(&event)?;
        debug!("user: {pubkey}, zapped {amount}");
        let zap_id = &String::from_utf8_lossy(&event.id).to_string();

        if self.get_user_zaps(&pubkey)?.contains(zap_id) {
            return Ok(());
        }

        self.update_account(&pubkey, amount.try_into().unwrap()).await?;
        self.add_zap(&pubkey, zap_id)?;

        Ok(())
    }

    pub fn get_all_accounts(&self) -> Result<(), Error> {
        self.db.lock().unwrap().read_all_accounts()
    }
}

pub fn read_zap(event: &Event) -> Result<(String, u64), Error> {
    let mut pubkey: Option<String> = None;
    let mut amount: Option<u64> = None;
    for t in &event.tags {
        let t = &t.values;

        if t[0].eq("p") {
            pubkey = Some(t[1].clone());
        }

        if t[0].eq("bolt11") {
            let invoice = str::parse::<Invoice>(&t[1])?;
            amount = invoice.amount_milli_satoshis();
        }
        if let (Some(pubkey), Some(amount)) = (pubkey.clone(), amount) {
            return Ok((pubkey, amount / 1000));
        }
    }

    Err(Error::NotFound)
}

#[cfg(test)]
mod tests {

    //use serial_test::serial;

    //use super::*;

    //  #[test]
    // #[serial]
}
