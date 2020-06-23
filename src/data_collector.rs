use anyhow::Result;
use sqlx::prelude::*;
use sqlx::query;
use sqlx::sqlite::Sqlite;
use sqlx::{SqliteConnection, SqlitePool};
use std::time::Duration;

#[derive(Clone)]
pub struct Collector {
    pool: SqlitePool,
}

impl Collector {
    pub async fn new() -> Result<Self> {
        let pool = sqlx::sqlite::SqlitePool::builder()
            .max_size(1)
            .connect_timeout(Duration::from_secs(5))
            .min_size(1)
            .build("sqlite://./passwords.db")
            .await?;
        query!("create table if not exists passwords(id integer primary key , password TEXT, count integer)").execute(&pool).await?;
        query!(
            "create table if not exists logins(id integer primary key , login TEXT, count integer)"
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }
    pub async fn log_password(self, pasword: &str, username: &str) -> Result<()> {
        let password_previous_count: i32 =
            match query!("select count from passwords where password = ?", &pasword)
                .fetch_one(&self.pool)
                .await
            {
                Ok(a) => match a.count {
                    Some(a) => a,
                    None => 0,
                },
                Err(e) => 0,
            };
        let login_previous_count: i32 =
            match query!("select count from logins where login = ?", &username)
                .fetch_one(&self.pool)
                .await
            {
                Ok(a) => match a.count {
                    Some(a) => a,
                    None => 0,
                },
                Err(e) => 0,
            };
        query!(
            "insert or replace into passwords VALUES (0, $1, $2)",
            &pasword,
            password_previous_count + 1
        )
        .execute(&self.pool)
        .await?;
        query!(
            "insert or replace into logins VALUES (0, $1, $2)",
            &username,
            login_previous_count + 1
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
