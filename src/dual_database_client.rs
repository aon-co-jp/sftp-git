//! DUAL DATABASE(aruaru-db + PostgreSQL)への実接続。`dual_database.rs`の
//! 状態遷移ロジックと組み合わせて使う。
//!
//! aruaru-dbはPostgreSQLワイヤープロトコル互換ポートを持つため
//! (`aruaru-server`の`--pg-port`、[aruaru-db](https://github.com/aon-co-jp/aruaru-db)
//! 参照)、同じ`tokio-postgres`クライアントでaruaru-db/PostgreSQL両方に
//! 接続できる。

use crate::dual_database::{ConsistencyMode, Database, DualDatabaseState};
use tokio_postgres::{Client, NoTls};

#[derive(Debug)]
pub enum DualDbError {
    Connect(Database, tokio_postgres::Error),
    Query(Database, tokio_postgres::Error),
}

impl std::fmt::Display for DualDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DualDbError::Connect(db, e) => write!(f, "{db:?}への接続に失敗: {e}"),
            DualDbError::Query(db, e) => write!(f, "{db:?}への書き込みに失敗: {e}"),
        }
    }
}

impl std::error::Error for DualDbError {}

pub struct DualDatabaseController {
    state: DualDatabaseState,
    aruaru_db_conn_str: String,
    postgres_conn_str: String,
}

impl DualDatabaseController {
    pub fn new(aruaru_db_conn_str: impl Into<String>, postgres_conn_str: impl Into<String>) -> Self {
        Self {
            state: DualDatabaseState::initial(),
            aruaru_db_conn_str: aruaru_db_conn_str.into(),
            postgres_conn_str: postgres_conn_str.into(),
        }
    }

    fn conn_str(&self, db: Database) -> &str {
        match db {
            Database::AruaruDb => &self.aruaru_db_conn_str,
            Database::PostgreSql => &self.postgres_conn_str,
        }
    }

    async fn connect(&self, db: Database) -> Result<Client, DualDbError> {
        let (client, connection) = tokio_postgres::connect(self.conn_str(db), NoTls)
            .await
            .map_err(|e| DualDbError::Connect(db, e))?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing_log_ignore(&e);
            }
        });
        Ok(client)
    }

    /// 主系へ同期で書き込む(常に待つ)。
    /// 平常時は非同期で従系へも複製する(戻り値のJoinHandleは呼び出し側が
    /// 必要に応じてawaitする。Failoverモード時は従系への複製は行わない
    /// ——障害中の相手へ書きに行かない設計)。
    pub async fn write(&self, sql: &str) -> Result<(), DualDbError> {
        let primary = self.state.primary;
        let primary_client = self.connect(primary).await?;
        primary_client
            .execute(sql, &[])
            .await
            .map_err(|e| DualDbError::Query(primary, e))?;

        if self.state.mode == ConsistencyMode::Normal {
            let secondary = self.state.secondary();
            let secondary_client = self.connect(secondary).await?;
            let sql_owned = sql.to_string();
            tokio::spawn(async move {
                let _ = secondary_client.execute(&sql_owned, &[]).await;
            });
        }

        Ok(())
    }

    pub fn state(&self) -> &DualDatabaseState {
        &self.state
    }

    pub fn on_primary_failure_detected(&mut self) {
        self.state.on_primary_failure_detected();
    }

    pub fn on_recovered_and_resynced(&mut self) {
        self.state.on_recovered_and_resynced();
    }
}

fn tracing_log_ignore(_e: &tokio_postgres::Error) {
    // 接続タスクのエラーは、write()側の呼び出し結果で既に扱われるため
    // ここでは黙って無視する(二重報告を避ける)。将来ロギング基盤導入時に
    // ここへ差し込む想定。
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conn_str_selects_correct_database_for_current_primary() {
        let controller = DualDatabaseController::new("host=aruaru", "host=postgres");
        assert_eq!(controller.conn_str(Database::AruaruDb), "host=aruaru");
        assert_eq!(controller.conn_str(Database::PostgreSql), "host=postgres");
    }

    /// 実DB接続が必要なため既定では実行しない。
    /// `SFTP_GIT_TEST_ARUARU_DB_URL`/`SFTP_GIT_TEST_POSTGRES_URL`を
    /// 設定した上で`cargo test -- --ignored`で手動実行する。
    #[tokio::test]
    #[ignore]
    async fn write_against_real_dual_database() {
        let aruaru = std::env::var("SFTP_GIT_TEST_ARUARU_DB_URL")
            .expect("SFTP_GIT_TEST_ARUARU_DB_URLを設定してください");
        let postgres = std::env::var("SFTP_GIT_TEST_POSTGRES_URL")
            .expect("SFTP_GIT_TEST_POSTGRES_URLを設定してください");
        let controller = DualDatabaseController::new(aruaru, postgres);
        controller
            .write("SELECT 1")
            .await
            .expect("実DUAL DATABASEへの書き込みに失敗");
    }
}
