//! Database operations for upstream/channel configuration
//!
//! This crate handles all database operations related to upstream services
//! (also known as channels in the API layer).
//!
//! API keys are encrypted at rest using AES-256-GCM. The master key is read
//! from the `MASTER_KEY` environment variable (64 hex chars = 32 bytes).
//! Encrypted values are stored with the prefix `aes256gcm:` followed by
//! hex-encoded nonce (12 bytes) + ciphertext. Plaintext values (legacy or
//! `sk-demo` placeholders) are returned as-is for backward compatibility.

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm, Nonce,
};
use burncloud_common::CrudRepository;
use burncloud_database::{phs, Database, DatabaseError, Result};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

const ENCRYPTED_PREFIX: &str = "aes256gcm:";

/// Upstream service configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RouterUpstream {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub match_path: String,
    pub auth_type: String, // Stored as string: "Bearer", "XApiKey"
    #[sqlx(default)]
    pub priority: i32,
    #[sqlx(default)]
    pub protocol: String, // "openai", "gemini", "claude"
    pub param_override: Option<String>,
    pub header_override: Option<String>,
    #[sqlx(default)]
    pub api_version: Option<String>,
}

/// Read and validate the master key from the `MASTER_KEY` environment variable.
/// Returns a clean error (not a panic) if the variable is missing or malformed.
pub fn get_master_key() -> Result<[u8; 32]> {
    let hex_key = std::env::var("MASTER_KEY").map_err(|_| {
        DatabaseError::Query(
            "MASTER_KEY environment variable is not set. \
             Set it to a 64-character hex string (32 bytes) before starting the server."
                .to_string(),
        )
    })?;
    let bytes = hex::decode(hex_key.trim())
        .map_err(|e| DatabaseError::Query(format!("MASTER_KEY is not valid hex: {}", e)))?;
    bytes.try_into().map_err(|_| {
        DatabaseError::Query("MASTER_KEY must be exactly 32 bytes (64 hex characters)".to_string())
    })
}

fn encrypt_api_key(plaintext: &str, key: &[u8; 32]) -> Result<String> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| DatabaseError::Query(format!("Failed to initialize cipher: {}", e)))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| DatabaseError::Query(format!("API key encryption failed: {}", e)))?;

    // nonce (12 bytes) || ciphertext, stored as hex with recognizable prefix
    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(format!("{}{}", ENCRYPTED_PREFIX, hex::encode(&combined)))
}

fn decrypt_api_key(stored: &str, key: &[u8; 32]) -> Result<String> {
    // Backward compat: values without the prefix are plaintext (legacy or demo)
    if !stored.starts_with(ENCRYPTED_PREFIX) {
        return Ok(stored.to_string());
    }

    let hex_data = &stored[ENCRYPTED_PREFIX.len()..];
    let combined = hex::decode(hex_data)
        .map_err(|e| DatabaseError::Query(format!("Malformed encrypted API key: {}", e)))?;

    if combined.len() < 12 {
        return Err(DatabaseError::Query(
            "Encrypted API key is too short (corrupted?)".to_string(),
        ));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| DatabaseError::Query(format!("Failed to initialize cipher: {}", e)))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
        DatabaseError::Query("API key decryption failed (wrong key or corrupted data)".to_string())
    })?;

    String::from_utf8(plaintext)
        .map_err(|e| DatabaseError::Query(format!("Decrypted API key is not valid UTF-8: {}", e)))
}

pub struct RouterUpstreamModel;

impl RouterUpstreamModel {
    /// Get all upstreams (api_key is decrypted before returning)
    pub async fn get_all(db: &Database) -> Result<Vec<RouterUpstream>> {
        let key = get_master_key()?;
        let conn = db.get_connection()?;
        let rows = sqlx::query_as::<_, RouterUpstream>(
            "SELECT id, name, base_url, api_key, match_path, auth_type, priority, protocol, param_override, header_override, api_version FROM router_upstreams"
        )
        .fetch_all(conn.pool())
        .await?;

        rows.into_iter()
            .map(|mut u| {
                u.api_key = decrypt_api_key(&u.api_key, &key)?;
                Ok(u)
            })
            .collect()
    }

    /// Get a single upstream by ID (api_key is decrypted before returning)
    pub async fn get(db: &Database, id: &str) -> Result<Option<RouterUpstream>> {
        let key = get_master_key()?;
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT id, name, base_url, api_key, match_path, auth_type, priority, protocol, param_override, header_override, api_version FROM router_upstreams WHERE id = $1"
        } else {
            "SELECT id, name, base_url, api_key, match_path, auth_type, priority, protocol, param_override, header_override, api_version FROM router_upstreams WHERE id = ?"
        };
        let upstream = sqlx::query_as::<_, RouterUpstream>(sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await?;

        upstream
            .map(|mut u| {
                u.api_key = decrypt_api_key(&u.api_key, &key)?;
                Ok(u)
            })
            .transpose()
    }

    /// Create a new upstream (api_key is encrypted before writing)
    pub async fn create(db: &Database, u: &RouterUpstream) -> Result<()> {
        let key = get_master_key()?;
        let encrypted_key = encrypt_api_key(&u.api_key, &key)?;

        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = format!(
            "INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type, priority, protocol, param_override, header_override, api_version) VALUES ({})",
            phs(is_postgres, 11)
        );
        sqlx::query(&sql)
            .bind(&u.id)
            .bind(&u.name)
            .bind(&u.base_url)
            .bind(&encrypted_key)
            .bind(&u.match_path)
            .bind(&u.auth_type)
            .bind(u.priority)
            .bind(&u.protocol)
            .bind(&u.param_override)
            .bind(&u.header_override)
            .bind(&u.api_version)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    /// Update an existing upstream (api_key is encrypted before writing)
    pub async fn update(db: &Database, u: &RouterUpstream) -> Result<()> {
        let key = get_master_key()?;
        let encrypted_key = encrypt_api_key(&u.api_key, &key)?;

        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "UPDATE router_upstreams SET name=$1, base_url=$2, api_key=$3, match_path=$4, auth_type=$5, priority=$6, protocol=$7, param_override=$8, header_override=$9, api_version=$10 WHERE id=$11"
        } else {
            "UPDATE router_upstreams SET name=?, base_url=?, api_key=?, match_path=?, auth_type=?, priority=?, protocol=?, param_override=?, header_override=?, api_version=? WHERE id=?"
        };
        sqlx::query(sql)
            .bind(&u.name)
            .bind(&u.base_url)
            .bind(&encrypted_key)
            .bind(&u.match_path)
            .bind(&u.auth_type)
            .bind(u.priority)
            .bind(&u.protocol)
            .bind(&u.param_override)
            .bind(&u.header_override)
            .bind(&u.api_version)
            .bind(&u.id)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    /// Delete an upstream
    pub async fn delete(db: &Database, id: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "DELETE FROM router_upstreams WHERE id = $1"
        } else {
            "DELETE FROM router_upstreams WHERE id = ?"
        };
        sqlx::query(sql).bind(id).execute(conn.pool()).await?;
        Ok(())
    }
}

/// Repository wrapper that implements the standard [`CrudRepository`] contract for upstreams.
///
/// `update` uses the `id` parameter as the authoritative ID (ignoring `input.id`).
/// `create` returns the entity that was stored (with the potentially encrypted api_key
/// replaced by its decrypted form, mirroring what `get` returns).
pub struct RouterUpstreamRepository<'a>(pub &'a Database);

#[async_trait::async_trait]
impl<'a> CrudRepository<RouterUpstream, String, DatabaseError> for RouterUpstreamRepository<'a> {
    async fn find_by_id(&self, id: &String) -> Result<Option<RouterUpstream>> {
        RouterUpstreamModel::get(self.0, id).await
    }

    async fn list(&self) -> Result<Vec<RouterUpstream>> {
        RouterUpstreamModel::get_all(self.0).await
    }

    async fn create(&self, input: &RouterUpstream) -> Result<RouterUpstream> {
        RouterUpstreamModel::create(self.0, input).await?;
        // Return the persisted entity. If the caller wants the decrypted key back,
        // fetch it through get() which goes through decrypt_api_key.
        RouterUpstreamModel::get(self.0, &input.id)
            .await?
            .ok_or_else(|| DatabaseError::Query("upstream disappeared after insert".to_string()))
    }

    async fn update(&self, id: &String, input: &RouterUpstream) -> Result<bool> {
        // Build a copy with the canonical ID so the UPDATE WHERE clause is correct.
        let mut record = input.clone();
        record.id = id.clone();
        RouterUpstreamModel::update(self.0, &record).await?;
        Ok(true)
    }

    async fn delete(&self, id: &String) -> Result<bool> {
        // Check existence first so we can return false when the record is not found.
        let exists = RouterUpstreamModel::get(self.0, id).await?.is_some();
        if exists {
            RouterUpstreamModel::delete(self.0, id).await?;
        }
        Ok(exists)
    }
}
