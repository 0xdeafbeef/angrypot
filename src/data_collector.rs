use anyhow::Result;
use futures::{StreamExt, TryStreamExt};
use log::{error, info};
use sqlx::query;
use sqlx::SqlitePool;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;

#[derive(Debug)]
pub struct Collector {
    pool: SqlitePool,
    receiver: Receiver<DbLogTypes>,
}
pub enum DbLogTypes {
    Password(String),
    Login(String),
    IpAddress(String)
}
impl Collector {
    pub async fn new(rx: Receiver<DbLogTypes>) -> Result<Self> {
        let pool = sqlx::sqlite::SqlitePool::builder()
            .max_size(1)
            .connect_timeout(Duration::from_secs(1))
            .min_size(1)
            .build("sqlite://./passwords.db")
            .await?;
        query!("create table if not exists passwords(password TEXT, count integer)")
            .execute(&pool)
            .await?;
        query!("create table if not exists logins(login TEXT, count integer)")
            .execute(&pool)
            .await?;
        Ok(Self { pool, receiver: rx })
    }
    pub async fn run(&mut self) -> Result<(), std::io::Error> {
        while let Some(data) = self.receiver.next().await {
            match data {
                DbLogTypes::EndOfCommunication => break,
                DbLogTypes::Login(a) => {
                    if let Err(e) = &self.save_login(&a).await {
                        error!("Error saving login in db: {}", e);
                    }
                }
                DbLogTypes::Password(a) => {
                    if let Err(e) = &self.save_password(&a).await {
                        error!("Error saving password in db: {}", e)
                    }
                }
            };
        }
        info!("Going out of collector run :(");
        Ok(())
    }
    async fn save_login(&self, login: &str) -> Result<()> {
        let login_previous_count: i32 =
            match query!("select count from logins where login = ?", &login)
                .fetch_one(&self.pool)
                .await
            {
                Ok(a) => match a.count {
                    Some(a) => a,
                    None => 0,
                },
                Err(_e) => 0,
            };
        info!("Saving login to db");
        query!(
            "insert or replace into logins VALUES ( ?, ?)",
            &login,
            login_previous_count + 1
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    async fn save_password(&self, pasword: &str) -> Result<()> {
        let password_previous_count: i32 =
            match query!("select count from passwords where password = ?", &pasword)
                .fetch_one(&self.pool)
                .await
            {
                Ok(a) => match a.count {
                    Some(a) => a,
                    None => 0,
                },
                Err(_e) => 0,
            };
        info!("Saving password to db");
        query!(
            "insert or replace into passwords VALUES ( ?, ?)",
            &pasword,
            password_previous_count + 1
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
