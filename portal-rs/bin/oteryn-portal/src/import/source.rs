use super::*;
use sqlx::{ConnectOptions, Executor};
use sqlx_mysql::{MySqlConnectOptions, MySqlConnection};

pub(super) type Dataset = BTreeMap<String, Vec<Record>>;
pub(super) trait Source {
    async fn snapshot(self, tables: &BTreeSet<&str>) -> Result<Dataset>;
}
pub(super) struct MysqlSource(MySqlConnection);
impl MysqlSource {
    pub async fn open(url: &str) -> Result<Self> {
        let options = url
            .parse::<MySqlConnectOptions>()
            .map_err(|_| "invalid source URL")?
            .disable_statement_logging();
        let mut conn = MySqlConnection::connect_with(&options)
            .await
            .map_err(|_| "source connection failed")?;
        conn.execute("SET SESSION TRANSACTION READ ONLY")
            .await
            .map_err(|_| "source read-only setup failed")?;
        conn.execute("SET time_zone = '+00:00'")
            .await
            .map_err(|_| "source UTC setup failed")?;
        conn.execute("SET SESSION TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .await
            .map_err(|_| "source isolation setup failed")?;
        Ok(Self(conn))
    }
}
impl Source for MysqlSource {
    async fn snapshot(mut self, tables: &BTreeSet<&str>) -> Result<Dataset> {
        self.0
            .execute("START TRANSACTION WITH CONSISTENT SNAPSHOT, READ ONLY")
            .await
            .map_err(|_| "source snapshot failed")?;
        let mut data = Dataset::new();
        for table in tables {
            let engine: Option<String>=sqlx::query_scalar("SELECT ENGINE FROM information_schema.TABLES WHERE TABLE_SCHEMA=DATABASE() AND TABLE_NAME=?").bind(table).fetch_optional(&mut self.0).await.map_err(|_| "source table engine check failed")?.flatten();
            if engine.as_deref().is_some_and(|e| e != "InnoDB") {
                return Err("source snapshot requires transactional tables");
            }

            let columns: Vec<String> = sqlx::query_scalar("SELECT COLUMN_NAME FROM information_schema.COLUMNS WHERE TABLE_SCHEMA=DATABASE() AND TABLE_NAME=? ORDER BY ORDINAL_POSITION")
                .bind(table).fetch_all(&mut self.0).await.map_err(|_| "source schema read failed")?;
            if columns.is_empty() {
                continue;
            }
            // Metadata identifiers are quoted; values are always decoded as strings to retain
            // integer precision, date precision and ciphertext without driver conversions.
            if columns
                .iter()
                .any(|c| !c.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
            {
                return Err("unsupported source column identifier");
            }
            let fields = columns
                .iter()
                .map(|c| format!("'{c}',CAST(`{c}` AS CHAR)"))
                .collect::<Vec<_>>()
                .join(",");
            let order: Vec<_> = ["id", "identity_id", "role_id", "permission_id"]
                .into_iter()
                .filter(|key| columns.iter().any(|c| c == key))
                .map(|c| format!("`{c}`"))
                .collect();
            let order = if order.is_empty() {
                String::new()
            } else {
                format!(" ORDER BY {}", order.join(","))
            };
            let query = format!("SELECT CAST(JSON_OBJECT({fields}) AS CHAR) FROM `{table}`{order}");
            let rows: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(query))
                .fetch_all(&mut self.0)
                .await
                .map_err(|_| "source table read failed")?;
            let rows = rows
                .iter()
                .map(|r| serde_json::from_str::<Record>(r).map_err(|_| "invalid source record"))
                .collect::<Result<Vec<_>>>()?;
            data.insert((*table).to_owned(), rows);
        }
        self.0
            .execute("ROLLBACK")
            .await
            .map_err(|_| "source snapshot close failed")?;
        Ok(data)
    }
}
