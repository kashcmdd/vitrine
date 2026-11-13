//! Persistence.
//!
//! Two pools over one database file: a single-connection writer and a wider read
//! pool. SQLite serialises writes anyway, so the constraint moves into the pool
//! and `SQLITE_BUSY` stops being something the application has to handle.

use std::path::Path;
use std::time::Duration;

use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::ConnectOptions;

use crate::Result;

/// A handle to the application database.
///
/// Writes go through [`Db::writer`], reads through [`Db::reader`]. Every write is
/// therefore serialised by construction rather than by retry loops.
pub struct Db {
    read: SqlitePool,
    write: SqlitePool,
}

impl Db {
    /// Opens (creating if needed) and migrates the database at `path`.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let options = Self::options(path.as_ref()).create_if_missing(true);

        let write = SqlitePoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .connect_with(options.clone())
            .await?;

        sqlx::migrate!("./migrations").run(&write).await?;

        let read = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await?;

        Ok(Self { read, write })
    }

    /// An in-memory database for tests. Migrations still run, so tests exercise
    /// the real schema rather than a hand-written approximation.
    pub async fn open_in_memory() -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .shared_cache(true)
            .filename(":memory:")
            .foreign_keys(true);

        let write = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options.clone())
            .await?;
        sqlx::migrate!("./migrations").run(&write).await?;

        let read = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await?;

        Ok(Self { read, write })
    }

    fn options(path: &Path) -> SqliteConnectOptions {
        SqliteConnectOptions::new()
            .filename(path)
            // WAL + NORMAL is the correct durability trade for a local app: a
            // crash can lose the last transaction, never the database.
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            // Per-connection, therefore set here rather than in a migration.
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5))
            .disable_statement_logging()
    }

    pub fn reader(&self) -> &SqlitePool {
        &self.read
    }

    pub fn writer(&self) -> &SqlitePool {
        &self.write
    }

    pub async fn close(self) {
        self.read.close().await;
        self.write.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_apply_to_a_fresh_database() {
        let db = Db::open_in_memory().await.expect("opens");
        let systems: i64 = sqlx::query_scalar("select count(*) from systems")
            .fetch_one(db.reader())
            .await
            .expect("query");
        assert!(systems >= 5, "systems should be seeded, got {systems}");
    }

    #[tokio::test]
    async fn the_three_way_join_is_queryable() {
        // Games, runners and sessions must be joinable in one statement: if this
        // needs three round trips, the schema is wrong.
        let db = Db::open_in_memory().await.expect("opens");
        let rows: i64 = sqlx::query_scalar(
            "select count(*) from games g
             left join runners r on r.id = g.runner_id
             left join play_sessions s on s.game_id = g.id",
        )
        .fetch_one(db.reader())
        .await
        .expect("query");
        assert_eq!(rows, 0);
    }
}
