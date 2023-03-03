use nostr::event::tag::Tag;
use nostr_sdk::prelude::schnorr::Signature;
use nostr_sdk::prelude::*;

use crate::config::Settings;
use crate::nauthz_grpc::event::TagEntry;

use crate::error::Error;
use crate::utils::{create_client, handle_keys};

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use crate::nauthz_grpc;

#[derive(Clone)]
pub struct Nostr {
    client: Client,
    settings: Settings
}

impl Nostr {
    pub async fn new(settings: Settings) -> Result<Self, Error> {
        let keys = handle_keys(Some(settings.info.nostr_key.clone())).unwrap();

        let client = create_client(&keys, vec![settings.info.relay_url.clone()])
            .await
            .unwrap();

        Ok(Self { client, settings })
    }

    pub async fn send_admission_dm(&self, pubkey: &str) -> Result<(), Error> {
        
        let _ = self.client.send_direct_msg(XOnlyPublicKey::from_str(pubkey)?, &self.settings.info.admission_message).await?;

        Ok(())
    }
    
    pub async fn send_balance_dm(&self, pubkey: &str, balance: &u64) -> Result<(), Error> {

        let msg = format!("Your account balance is {balance}");
        
        let _ = self.client.send_direct_msg(XOnlyPublicKey::from_str(pubkey)?, msg).await?;

        Ok(())
    }
}

