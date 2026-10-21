use redis::{AsyncCommands, aio::ConnectionManager};
use serde::{Serialize, de::DeserializeOwned};

#[derive(Clone)]
pub struct CacheReqResp {
    conn: ConnectionManager,
}

impl CacheReqResp {
    // Connect to redis
    pub async fn connect(redis_url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let conn = client.get_connection_manager().await?;
        Ok(Self { conn })
    }

    // Read cached data JSON-encoded
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let mut conn = self.conn.clone();
        let raw: Option<String> = conn.get(key).await.ok()?;
        raw.and_then(|s| serde_json::from_str(&s).ok())
    }

    // Write to redis
    pub async fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_secs: u64,
    ) -> anyhow::Result<()> {
        let mut conn = self.conn.clone();
        let payload = serde_json::to_string(value)?;
        conn.set_ex::<_, _, ()>(key, payload, ttl_secs).await?;
        Ok(())
    }

    // Build cache key
    pub fn key_for(query: &str, id: &str) -> String {
        format!("api:v1:{}:{}", query.trim().to_lowercase(), id)
    }
}

