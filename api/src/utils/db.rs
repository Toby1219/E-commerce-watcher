use sea_orm::{Database, DatabaseConnection};

pub async fn connect_db(url:&str)->DatabaseConnection{
    Database::connect(url).await.expect("db connect failed")
}


