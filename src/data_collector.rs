use crate::data_collector::DbLogTypes::IpAddress;
use anyhow::{Error, Result};
use futures::StreamExt;
use influx_db_client::{Point, Points, UdpClient, Value};
use log::{debug, error, info};
use maxminddb::geoip2::Country;
use maxminddb::Reader;
use sqlx::query;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;

pub struct Collector {
    pool: SqlitePool,
    receiver: Receiver<DbLogTypes>,
    influxdb_client: UdpClient,
    geoip_data: maxminddb::Reader<Vec<u8>>,
}
pub enum DbLogTypes {
    Password(String),
    Login(String),
    IpAddress(String),
}
impl Collector {
    pub async fn new<T: AsRef<Path>>(
        rx: Receiver<DbLogTypes>,
        influx_addr: SocketAddr,
        geoip_path: T,
    ) -> Result<Self> {
        let pool = sqlx::sqlite::SqlitePool::builder()
            .max_size(1)
            .connect_timeout(Duration::from_secs(1))
            .min_size(1)
            .build("sqlite://./passwords.db")
            .await?;
        Ok(Self {
            pool,
            receiver: rx,
            influxdb_client: UdpClient::new(influx_addr),
            geoip_data: Reader::open_readfile(geoip_path)?,
        })
    }
    pub async fn run(&mut self) -> Result<(), std::io::Error> {
        while let Some(data) = self.receiver.next().await {
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
                    &self.send_metrics(a);
                }
            };
        }
        Ok(())
    }
    fn geo_ip_decode(db: &maxminddb::Reader<Vec<u8>>, address: IpAddr) -> Result<String> {
        Ok(db
            .lookup::<Country>(address)?
            .country
            .ok_or_else(|| Error::msg("Failed looking up for country"))?
            .iso_code
            .ok_or_else(|| Error::msg("Failed looking up for country code"))?
            .to_string())
    }
    fn send_metrics(&self, addr: String) -> Result<()> {
        let code = Self::geo_ip_decode(&self.geoip_data, addr.parse::<IpAddr>()?)?;
        let point = Point::new("ip_attack").add_field("ip", Value::String(addr)).add_tag("country",Value::String(code));
        self.influxdb_client.write_point(point)?;
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
#[cfg(test)]
mod tests {
    use crate::data_collector::Collector;
    use dotenv::var;
    use maxminddb::Reader;
    use std::net::IpAddr;
    use std::path::PathBuf;
    #[test]
    fn test_geo_ip_coding() {
        assert_eq!(
            "US",
            Collector::geo_ip_decode(
                &Reader::open_readfile(PathBuf::from(var("GEOIP_DB").unwrap())).unwrap(),
                IpAddr::from([64, 223, 164, 101])
            )
            .unwrap()
        );
    }
}
