use anyhow::Result;
use futures::StreamExt;
use influx_db_client::{Point, Points, UdpClient, Value};
use log::{error, info};
use sqlx::query;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;

pub struct Collector {
    pool: SqlitePool,
    receiver: Receiver<DbLogTypes>,
    ip_storage: HashMap<String, usize>,
    influxdb_client: UdpClient,
}
pub enum DbLogTypes {
    Password(String),
    Login(String),
    IpAddress(String),
}
impl Collector {
    pub async fn new(rx: Receiver<DbLogTypes>, influx_addr: SocketAddr) -> Result<Self> {
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

        Ok(Self {
            pool,
            receiver: rx,
            ip_storage: HashMap::new(),
            influxdb_client: UdpClient::new(influx_addr),
        })
    }
    pub async fn run(&mut self) -> Result<(), std::io::Error> {
        let mut function_strart_time = tokio::time::Instant::now();
        while let Some(data) = self.receiver.next().await {
            let cycle_start_time = tokio::time::Instant::now();
            if cycle_start_time - function_strart_time > tokio::time::Duration::from_secs(60) {
                function_strart_time = cycle_start_time;
                if !&self.ip_storage.is_empty() {
                    if let Err(e) = self.send_metrics() {
                        error!("Error sending metrics to influxdb: {}", e);
                    };
                }
            };
            match data {
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
                DbLogTypes::IpAddress(a) => {
                    *self.ip_storage.entry(a).or_insert(0) += 1;
                }
            };
        }
        info!("Going out of collector run :(");
        Ok(())
    }

    fn send_metrics(&mut self) -> Result<(), influx_db_client::Error> {
        let mut batch = vec![];
        for (k, v) in &self.ip_storage {
            let pont = Point::new("ip_attack").add_field(k, Value::Integer(*v as i64));
            batch.push(pont);
        }
        self.influxdb_client
            .write_points(Points::create_new(batch))?;
        self.ip_storage.clear();
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
