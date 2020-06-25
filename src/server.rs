use crate::data_collector::DbLogTypes;
use futures::executor::block_on;
use log::{error, info};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thrussh::server::Response;
use thrussh::server::{Auth, Session};
use thrussh::*;
use thrussh_keys::key::PublicKey;
use tokio::sync::mpsc::Sender;
#[derive(Clone)]
pub struct Server {
    pub tx: Sender<DbLogTypes>,
    pub clients: Arc<Mutex<HashMap<(usize, ChannelId), thrussh::server::Handle>>>,
    pub id: usize,
}

impl server::Server for Server {
    type Handler = Self;
    fn new(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }
}

impl server::Handler for Server {
    type FutureAuth = futures::future::Ready<Result<(Self, server::Auth), anyhow::Error>>;
    type FutureUnit = futures::future::Ready<Result<(Self, Session), anyhow::Error>>;
    type FutureBool = futures::future::Ready<Result<(Self, Session, bool), anyhow::Error>>;

    fn finished_auth(self, auth: Auth) -> Self::FutureAuth {
        info!("finished_auth: {:?}", auth);
        futures::future::ready(Ok((self, auth)))
    }

    fn finished_bool(self, b: bool, s: Session) -> Self::FutureBool {
        info!("Finished bool");
        futures::future::ready(Ok((self, s, b)))
    }

    fn finished(self, session: Session) -> Self::FutureUnit {
        info!("finished");
        futures::future::ready(Ok((self, session)))
    }
    fn auth_none(self, user: &str) -> Self::FutureAuth {
        info!("auth_none {}", &user);
        futures::future::ready(Ok((self, server::Auth::Reject)))
    }
    fn auth_password(mut self, user: &str, password: &str) -> Self::FutureAuth {
        info!("auth password {}:{}", &user, &password);

        if let Err(e) = block_on(self.tx.send(DbLogTypes::Password(password.to_string()))) {
            error!("Error sending password to db: {}", e)
        };
        if let Err(e) = block_on(self.tx.send(DbLogTypes::Login(user.to_string()))) {
            error!("Error sending login to db: {}", e)
        }
        futures::future::ready(Ok((self, server::Auth::Reject)))
    }
    fn auth_publickey(self, user: &str, _publickey: &PublicKey) -> Self::FutureAuth {
        info!("auth_publickey {}", &user);
        futures::future::ready(Ok((self, server::Auth::Reject)))
    }
    fn auth_keyboard_interactive(
        self,
        user: &str,
        _submethods: &str,
        _response: Option<Response>,
    ) -> Self::FutureAuth {
        info!("auth_keyboard_interactive {}", &user);
        futures::future::ready(Ok((self, server::Auth::Reject)))
    }
    fn channel_close(self, _channel: ChannelId, _session: Session) -> Self::FutureUnit {
        info!("channel_close");
        futures::future::ready(Ok((self, _session)))
    }
    fn channel_eof(self, _channel: ChannelId, _session: Session) -> Self::FutureUnit {
        info!("channel_eof");
        futures::future::ready(Ok((self, _session)))
    }

    fn channel_open_session(
        self,
        _channel: ChannelId,
        session: server::Session,
    ) -> Self::FutureUnit {
        info!("Session opened!");
        futures::future::ready(Ok((self, session)))
    }
    fn data(self, channel: ChannelId, data: &[u8], session: server::Session) -> Self::FutureUnit {
        info!(
            "data on channel {:?}: {:?}",
            channel,
            std::str::from_utf8(data)
        );
        futures::future::ready(Ok((self, session)))
    }
}
