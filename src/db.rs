use std::collections::HashSet;

use redb::{
    Database, MultimapTableDefinition, ReadableMultimapTable, ReadableTable, TableDefinition,
};
use tracing::debug;

use crate::{config::Cost, error::Error};
// key is hex pubkey value is name
const ACCOUNTTABLE: TableDefinition<&str, u64> = TableDefinition::new("account");
const ZAPSTABLE: MultimapTableDefinition<&str, &str> = MultimapTableDefinition::new("zaps");

#[derive(Debug, PartialEq, Eq)]
pub struct Account {
    pub pubkey: String,
    pub balance: u64,
}

impl Account {
    pub fn is_admitted(&self, cost: Cost) -> bool {
        if self.balance.lt(&cost.admission) {
            return false;
        }

        if self.balance.lt(&(cost.per_event + cost.admission)) {
            return false;
        }

        true
    }
}

pub struct Db {
    db: Database,
}

impl Default for Db {
    fn default() -> Self {
        Self::new()
    }
}

impl Db {
    pub fn new() -> Self {
        debug!("Creating DB");
        let db = Database::create("my_db.redb").unwrap();
        //  db.set_write_strategy(WriteStrategy::TwoPhase).unwrap();
        let write_txn = db.begin_write().unwrap();
        {
            // Opens the table to create it
            let _ = write_txn.open_table(ACCOUNTTABLE).unwrap();
            let _ = write_txn.open_multimap_table(ZAPSTABLE).unwrap();
        }
        write_txn.commit().unwrap();

        Self { db }
    }

    pub fn write_account(&self, account: &Account) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(ACCOUNTTABLE)?;
            table.insert(account.pubkey.as_str(), account.balance)?;
        }
        write_txn.commit().unwrap();
        Ok(())
    }

    pub fn read_account(&self, pubkey: &str) -> Result<Option<Account>, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ACCOUNTTABLE)?;
        if let Some(account_info) = table.get(pubkey)? {
            let account = Account {
                pubkey: pubkey.to_string(),
                balance: account_info.value(),
            };
            return Ok(Some(account));
        }
        Ok(None)
    }

    pub fn read_all_accounts(&self) -> Result<(), Error> {
        debug!("Registered accounts");
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ACCOUNTTABLE)?;

        for a in table.iter()? {
            debug!("{:?}, {}", a.0.value(), a.1.value());
        }
        Ok(())
    }

    pub fn clear_tables(&self) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;

        {
            let mut table = write_txn.open_table(ACCOUNTTABLE)?;
            while table.len()? > 0 {
                let _ = table.pop_first();
            }

            let mut table = write_txn.open_multimap_table(ZAPSTABLE)?;
            let keys: HashSet<String> = table.iter()?.map(|(x, _)| x.value().to_string()).collect();

            for k in keys {
                table.remove_all(k.as_str())?;
            }
        }
        write_txn.commit().unwrap();

        Ok(())
    }

    pub fn write_zap(&self, pubkey: &str, zap_id: &str) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_multimap_table(ZAPSTABLE)?;
            table.insert(pubkey, zap_id)?;
        }
        write_txn.commit().unwrap();
        Ok(())
    }

    pub fn read_zap(&self, pubkey: &str) -> Result<HashSet<String>, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_multimap_table(ZAPSTABLE)?;

        let result = table.get(pubkey)?;

        Ok(result.map(|e| e.value().to_string()).collect())
    }
}

#[cfg(test)]
mod tests {
    //use crate::utils::unix_time;
    //use serial_test::serial;
}
