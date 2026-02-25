use burncloud_common::types::{TieredPrice, TieredPriceInput};
use burncloud_database::{Database, Result};

pub struct TieredPriceModel;

impl TieredPriceModel {
    /// Get all tiers for a model, optionally filtered by region
    pub async fn get_tiers(
        db: &Database,
        model: &str,
        region: Option<&str>,
    ) -> Result<Vec<TieredPrice>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let tiers = match region {
            Some(r) => {
                let sql = if is_postgres {
                    r#"SELECT id, model, region, tier_start, tier_end, input_price, output_price
                       FROM tiered_pricing WHERE model = $1 AND region = $2
                       ORDER BY tier_start ASC"#
                } else {
                    r#"SELECT id, model, region, tier_start, tier_end, input_price, output_price
                       FROM tiered_pricing WHERE model = ? AND region = ?
                       ORDER BY tier_start ASC"#
                };
                sqlx::query_as(sql)
                    .bind(model)
                    .bind(r)
                    .fetch_all(conn.pool())
                    .await?
            }
            None => {
                // Get tiers with NULL region (universal) or matching region
                let sql = if is_postgres {
                    r#"SELECT id, model, region, tier_start, tier_end, input_price, output_price
                       FROM tiered_pricing WHERE model = $1
                       ORDER BY tier_start ASC"#
                } else {
                    r#"SELECT id, model, region, tier_start, tier_end, input_price, output_price
                       FROM tiered_pricing WHERE model = ?
                       ORDER BY tier_start ASC"#
                };
                sqlx::query_as(sql)
                    .bind(model)
                    .fetch_all(conn.pool())
                    .await?
            }
        };

        Ok(tiers)
    }

    /// Upsert a tiered price
    pub async fn upsert_tier(db: &Database, input: &TieredPriceInput) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            r#"
            INSERT INTO tiered_pricing (model, region, tier_start, tier_end, input_price, output_price)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT(model, region, tier_start) DO UPDATE SET
                tier_end = EXCLUDED.tier_end,
                input_price = EXCLUDED.input_price,
                output_price = EXCLUDED.output_price
            "#
        } else {
            r#"
            INSERT INTO tiered_pricing (model, region, tier_start, tier_end, input_price, output_price)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(model, region, tier_start) DO UPDATE SET
                tier_end = excluded.tier_end,
                input_price = excluded.input_price,
                output_price = excluded.output_price
            "#
        };

        sqlx::query(sql)
            .bind(&input.model)
            .bind(&input.region)
            .bind(input.tier_start)
            .bind(input.tier_end)
            .bind(input.input_price)
            .bind(input.output_price)
            .execute(conn.pool())
            .await?;

        Ok(())
    }

    /// Delete all tiers for a model and region
    pub async fn delete_tiers(db: &Database, model: &str, region: Option<&str>) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        match region {
            Some(r) => {
                let sql = if is_postgres {
                    "DELETE FROM tiered_pricing WHERE model = $1 AND region = $2"
                } else {
                    "DELETE FROM tiered_pricing WHERE model = ? AND region = ?"
                };
                sqlx::query(sql)
                    .bind(model)
                    .bind(r)
                    .execute(conn.pool())
                    .await?;
            }
            None => {
                let sql = if is_postgres {
                    "DELETE FROM tiered_pricing WHERE model = $1"
                } else {
                    "DELETE FROM tiered_pricing WHERE model = ?"
                };
                sqlx::query(sql).bind(model).execute(conn.pool()).await?;
            }
        }

        Ok(())
    }

    /// Check if a model has tiered pricing configured
    pub async fn has_tiered_pricing(db: &Database, model: &str) -> Result<bool> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => "SELECT COUNT(*) FROM tiered_pricing WHERE model = $1",
            _ => "SELECT COUNT(*) FROM tiered_pricing WHERE model = ?",
        };

        let count: i64 = sqlx::query_scalar(sql)
            .bind(model)
            .fetch_one(conn.pool())
            .await?;

        Ok(count > 0)
    }

    /// List all tiered pricing entries
    pub async fn list_all(db: &Database) -> Result<Vec<TieredPrice>> {
        let conn = db.get_connection()?;
        let sql = r#"SELECT id, model, region, tier_start, tier_end, input_price, output_price
                     FROM tiered_pricing ORDER BY model, tier_start ASC"#;

        let tiers = sqlx::query_as(sql).fetch_all(conn.pool()).await?;

        Ok(tiers)
    }
}
