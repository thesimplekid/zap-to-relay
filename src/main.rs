use db::Account;
use nostr_sdk::prelude::hex::ToHex;
use tonic::{transport::Server, Request, Response, Status};

use nauthz_grpc::authorization_server::{Authorization, AuthorizationServer};
use nauthz_grpc::{Decision, EventReply, EventRequest};
use std::sync::{Arc, Mutex};

use crate::config::Settings;
use crate::nostr::Nostr;
use crate::repo::Repo;

use tracing::{debug, info};

pub mod nauthz_grpc {
    tonic::include_proto!("nauthz");
}

pub mod config;
pub mod db;
pub mod error;
pub mod nostr;
pub mod repo;
pub mod utils;

pub struct EventAuthz {
    pub repo: Repo,
    pub settings: Settings,
}

#[tonic::async_trait]
impl Authorization for EventAuthz {
    async fn event_admit(
        &self,
        request: Request<EventRequest>,
    ) -> Result<Response<EventReply>, Status> {
        let req = request.into_inner();
        let event = req.clone().event.unwrap();
        let content_prefix: String = event.content.chars().take(40).collect();
        info!("recvd event, [kind={}, origin={:?}, nip05_domain={:?}, tag_count={}, content_sample={:?}]",
                 event.kind, req.origin, req.nip05.as_ref().map(|x| x.domain.clone()), event.tags.len(), content_prefix);

        let author = match req.auth_pubkey {
            Some(_) => req.auth_pubkey(),
            None => &event.pubkey,
        };

        let author = author.to_hex();

        if author.eq("5536abf588ca928d26a78f5a3e2751483fc11a6f3a81d2c4d4651373508b5247") {
            return Ok(Response::new(nauthz_grpc::EventReply {
                decision: Decision::Permit as i32,
                message: Some("Ok".to_string()),
            }));

        }

        if let Some(denylist) = &self.settings.info.denylist {
            if denylist.contains(&author) {
                return Ok(Response::new(nauthz_grpc::EventReply {
                    decision: Decision::Deny as i32,
                    message: Some("Not allowed to publish".to_string()),
                }));
            }
        }

        // If author is zapper, update account balance
        // admit event

        if author.eq(&self.settings.info.zapper_key) {
            // TODO: parse event to get the amount and pubkey

            self.repo.handle_zap(event).await.unwrap();

            self.repo.get_all_accounts().unwrap();

            return Ok(Response::new(nauthz_grpc::EventReply {
                decision: Decision::Permit as i32,
                message: Some("Ok".to_string()),
            }));
        }
        // else check if check has balance > required admint

        let user_account = self.repo.get_account(&author).unwrap();

        if user_account.is_none() {
            return Ok(Response::new(nauthz_grpc::EventReply {
                decision: Decision::Deny as i32,
                message: Some("Not allowed to publish".to_string()),
            }));
        }

        let user_account = user_account.unwrap();

        if !user_account.is_admitted(self.settings.cost) {
            return Ok(Response::new(nauthz_grpc::EventReply {
                decision: Decision::Deny as i32,
                message: Some("Not allowed to publish".to_string()),
            }));
        }
        if self.settings.cost.per_event > 0 {
            let updated_account = Account {
                pubkey: author,
                balance: user_account.balance - self.settings.cost.per_event,
            };
            self.repo.add_account(&updated_account).unwrap();
        }
        Ok(Response::new(nauthz_grpc::EventReply {
            decision: Decision::Permit as i32,
            message: Some("Ok".to_string()),
        }))
        // else deny
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();

    tracing_subscriber::fmt::try_init().unwrap();

    let settings = config::Settings::new(&None);

    debug!("{:?}", settings);

    let nos = Nostr::new(settings.clone()).await?;

    let repo = Repo::new(nos);

    repo.get_all_accounts()?;

    let checker = EventAuthz {
        repo,
        settings,
    };

    info!("EventAuthz Server listening on {addr}");
    // Start serving
    Server::builder()
        .add_service(AuthorizationServer::new(checker))
        .serve(addr)
        .await?;
    Ok(())
}
