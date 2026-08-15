//! DUAL DATABASE(aruaru-db + PostgreSQL)整合性モデル。DESIGN.md「4.」参照。
//!
//! 平常時: aruaru-db(主)へ同期確定+PostgreSQL(従)へ非同期レプリケーション
//! (結果整合、低レイテンシ)。
//! 障害検知時: PostgreSQLを主系へ一時昇格し、同期確定(強整合、
//! データ無損失優先)へ切り替える。
//!
//! ここでは実際のDB接続は行わず、状態遷移(モード切り替え)ロジックのみを
//! 実装・テストする。実DB接続は次回、DBプロキシ層の設計と合わせて行う。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Database {
    AruaruDb,
    PostgreSql,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyMode {
    /// 平常時: 主系へ同期、従系へ非同期(結果整合)
    Normal,
    /// 障害時: 昇格した主系へ同期確定(強整合)
    Failover,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualDatabaseState {
    pub primary: Database,
    pub mode: ConsistencyMode,
}

impl DualDatabaseState {
    /// 平常時の初期状態: aruaru-dbが主系。
    pub fn initial() -> Self {
        Self {
            primary: Database::AruaruDb,
            mode: ConsistencyMode::Normal,
        }
    }

    /// 主系の障害を検知した際に呼ぶ。もう片方を主系に昇格し、
    /// 強整合(Failover)モードへ切り替える。既にFailoverモードの場合は
    /// 変化なし(二重昇格を防ぐ)。
    pub fn on_primary_failure_detected(&mut self) {
        if self.mode == ConsistencyMode::Failover {
            return;
        }
        self.primary = other(self.primary);
        self.mode = ConsistencyMode::Failover;
    }

    /// 元の主系(aruaru-db)が復旧した際に呼ぶ。差分再同期が完了した
    /// 前提で、平常時モードへ戻す。
    pub fn on_recovered_and_resynced(&mut self) {
        self.primary = Database::AruaruDb;
        self.mode = ConsistencyMode::Normal;
    }

    pub fn secondary(&self) -> Database {
        other(self.primary)
    }
}

fn other(db: Database) -> Database {
    match db {
        Database::AruaruDb => Database::PostgreSql,
        Database::PostgreSql => Database::AruaruDb,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_aruaru_db_primary_normal_mode() {
        let state = DualDatabaseState::initial();
        assert_eq!(state.primary, Database::AruaruDb);
        assert_eq!(state.mode, ConsistencyMode::Normal);
        assert_eq!(state.secondary(), Database::PostgreSql);
    }

    #[test]
    fn failure_detected_promotes_postgresql_and_switches_to_failover_mode() {
        let mut state = DualDatabaseState::initial();
        state.on_primary_failure_detected();
        assert_eq!(state.primary, Database::PostgreSql);
        assert_eq!(state.mode, ConsistencyMode::Failover);
    }

    #[test]
    fn repeated_failure_detection_does_not_flip_primary_again() {
        let mut state = DualDatabaseState::initial();
        state.on_primary_failure_detected();
        state.on_primary_failure_detected();
        assert_eq!(state.primary, Database::PostgreSql);
        assert_eq!(state.mode, ConsistencyMode::Failover);
    }

    #[test]
    fn recovery_restores_aruaru_db_as_primary_and_normal_mode() {
        let mut state = DualDatabaseState::initial();
        state.on_primary_failure_detected();
        state.on_recovered_and_resynced();
        assert_eq!(state.primary, Database::AruaruDb);
        assert_eq!(state.mode, ConsistencyMode::Normal);
    }
}
