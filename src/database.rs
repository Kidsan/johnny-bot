use rusqlite::params;
use tokio_rusqlite::Connection;

use crate::Error;

pub trait BalanceDatabase {
    async fn get_balance(&self, user_id: String) -> Result<i32, Error>;
    async fn set_balance(&self, user_id: String, balance: i32) -> Result<(), Error>;
    async fn get_leaderboard(&self) -> Result<Vec<(String, i32)>, Error>;
}

pub struct Database {
    connection: tokio_rusqlite::Connection,
}

impl Database {
    pub async fn new() -> Result<Self, Error> {
        let db = Connection::open("johnny.db").await.unwrap();
        db.call(|conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS balances (
            id TEXT PRIMARY KEY,
            balance INTEGER NOT NULL
       )",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
        Ok(Self { connection: db })
    }
}

impl BalanceDatabase for Database {
    async fn get_balance(&self, user_id: String) -> Result<i32, Error> {
        let user = user_id.clone();
        let balance = self
            .connection
            .call(move |conn| {
                let mut stmt =
                    conn.prepare_cached("SELECT balance FROM balances WHERE id = (?1)")?;
                Ok(stmt.query_row(params![user], |row| {
                    let balance: i32 = row.get(0)?;

                    Ok(balance)
                }))
            })
            .await?;
        let result = match balance {
            Ok(user_balance) => user_balance,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                let user = user_id;
                let _ = self
                    .connection
                    .call(move |conn| {
                        let mut stmt = conn
                            .prepare_cached("INSERT INTO balances (id, balance) VALUES (?1, ?2)")?;
                        Ok(stmt.query_row(params![user, 50], |row| {
                            let balance: i32 = row.get(0)?;

                            Ok(balance)
                        }))
                    })
                    .await?;
                50
            }
            Err(e) => return Err(e.into()),
        };
        Ok(result)
    }

    async fn get_leaderboard(&self) -> Result<Vec<(String, i32)>, Error> {
        let leaderboard = self
            .connection
            .call(|conn| {
                let mut stmt = conn.prepare_cached(
                    "SELECT id, balance FROM balances ORDER BY balance DESC LIMIT 10",
                )?;
                let people = stmt
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect::<std::result::Result<Vec<(String, i32)>, rusqlite::Error>>();
                Ok(people)
            })
            .await
            .unwrap()?;
        Ok(leaderboard)
    }

    async fn set_balance(&self, user_id: String, balance: i32) -> Result<(), Error> {
        let _ = self
            .connection
            .call(move |conn| {
                let mut stmt =
                    conn.prepare_cached("UPDATE balances SET balance = (?1) WHERE id = (?2)")?;
                Ok(stmt.execute(params![balance, user_id]))
            })
            .await?;
        Ok(())
    }
}
