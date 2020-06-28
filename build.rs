use sqlx::query;
use std::time::Duration;
use tokio::prelude::*;

#[tokio::main]
async fn main() {
    let pool = sqlx::sqlite::SqlitePool::builder()
        .max_size(1)
        .connect_timeout(Duration::from_secs(1))
        .min_size(1)
        .build("sqlite://./passwords.db")
        .await
        .expect("Error connection to db while build");
    query("create table if not exists passwords(password TEXT, count integer)")
        .execute(&pool)
        .await
        .expect("Error creating db while building");
    query("create table if not exists logins(login TEXT, count integer)")
        .execute(&pool)
        .await
        .expect("Error creating db while building");
}
