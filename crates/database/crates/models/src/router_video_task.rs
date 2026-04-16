use burncloud_database::{Database, Result};

#[derive(Debug, Clone)]
pub struct RouterVideoTask {
    pub task_id: String,
    pub channel_id: i32,
    pub user_id: Option<String>,
    pub model: Option<String>,
    pub duration: i64,
    pub resolution: String,
}

pub struct RouterVideoTaskModel;

impl RouterVideoTaskModel {
    /// Persist a video task mapping (task_id → channel_id) for later GET routing.
    pub async fn save(db: &Database, task: &RouterVideoTask) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => {
                r#"
                INSERT INTO router_video_tasks (task_id, channel_id, user_id, model, duration, resolution)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (task_id) DO NOTHING
                "#
            }
            _ => {
                r#"
                INSERT OR IGNORE INTO router_video_tasks (task_id, channel_id, user_id, model, duration, resolution)
                VALUES (?, ?, ?, ?, ?, ?)
                "#
            }
        };

        sqlx::query(sql)
            .bind(&task.task_id)
            .bind(task.channel_id)
            .bind(&task.user_id)
            .bind(&task.model)
            .bind(task.duration)
            .bind(&task.resolution)
            .execute(conn.pool())
            .await?;

        Ok(())
    }

    /// Look up a video task by task_id for GET /v1/videos/{task_id} routing.
    pub async fn get_by_task_id(db: &Database, task_id: &str) -> Result<Option<RouterVideoTask>> {
        let conn = db.get_connection()?;
        let sql = match db.kind().as_str() {
            "postgres" => {
                r#"SELECT task_id, channel_id, user_id, model, duration, resolution
                   FROM router_video_tasks WHERE task_id = $1"#
            }
            _ => {
                r#"SELECT task_id, channel_id, user_id, model, duration, resolution
                   FROM router_video_tasks WHERE task_id = ?"#
            }
        };

        let row = sqlx::query(sql)
            .bind(task_id)
            .fetch_optional(conn.pool())
            .await?;

        Ok(row.map(|r| {
            use sqlx::Row;
            RouterVideoTask {
                task_id: r.get("task_id"),
                channel_id: r.get("channel_id"),
                user_id: r.get("user_id"),
                model: r.get("model"),
                duration: r.get::<i64, _>("duration"),
                resolution: r.get("resolution"),
            }
        }))
    }
}
