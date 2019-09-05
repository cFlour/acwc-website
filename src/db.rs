use postgres::{Client, NoTls};
use r2d2::{Pool, PooledConnection};
use r2d2_postgres::PostgresConnectionManager;
use serde::Serialize;

pub const STATUS_PENDING: i32 = 0;
pub const STATUS_APPROVED: i32 = 1;
pub const STATUS_REJECTED: i32 = 2;

pub type DbPool = Pool<PostgresConnectionManager<NoTls>>;
type DbConnection = PooledConnection<PostgresConnectionManager<NoTls>>;

#[derive(Serialize)]
pub struct Registration {
    pub lichess_id: String,
    pub lichess_username: String,
    pub status: i32,
    pub registrant_comment: String,
    pub td_comment: String,
    pub special: bool,
}

pub fn connect(connection_string: &str) -> Result<Client, postgres::Error> {
    Client::connect(connection_string, NoTls)
}

fn set_status<T: AcwcDbClient>(
    db_client: &T,
    lichess_id: &str,
    td_comment: &str,
    status: i32,
) -> Result<u64, Box<dyn std::error::Error>> {
    Ok(db_client.w()?.execute(
        "UPDATE registrations SET status = $1, tdcomment = $2 WHERE lichessid = $3",
        &[&status, &td_comment, &lichess_id],
    )?)
}

pub trait AcwcDbClient {
    fn w(&self) -> Result<DbConnection, Box<dyn std::error::Error>>;
    fn insert_registration(
        &self,
        registration: &Registration,
    ) -> Result<u64, Box<dyn std::error::Error>>;
    fn find_registration(
        &self,
        lichess_id: &str,
    ) -> Result<Option<Registration>, Box<dyn std::error::Error>>;
    fn all_registrations(&self) -> Result<Vec<Registration>, Box<dyn std::error::Error>>;
    fn approve_registration(
        &self,
        lichess_id: &str,
        td_comment: &str,
    ) -> Result<u64, Box<dyn std::error::Error>>;
    fn reject_registration(
        &self,
        lichess_id: &str,
        td_comment: &str,
    ) -> Result<u64, Box<dyn std::error::Error>>;
    fn withdraw_registration(&self, lichess_id: &str) -> Result<u64, Box<dyn std::error::Error>>;
}

impl AcwcDbClient for DbPool {
    fn w(&self) -> Result<DbConnection, Box<dyn std::error::Error>> {
        Ok(self.get()?)
    }

    fn insert_registration(
        &self,
        registration: &Registration,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        Ok(self.w()?.execute(
            "INSERT INTO registrations (lichessid, lichessusername, status, \
             registrantcomment, tdcomment, special) VALUES ($1, $2, $3, $4, $5, $6)",
            &[
                &registration.lichess_id,
                &registration.lichess_username,
                &registration.status,
                &registration.registrant_comment,
                &registration.td_comment,
                &registration.special,
            ],
        )?)
    }

    fn find_registration(
        &self,
        lichess_id: &str,
    ) -> Result<Option<Registration>, Box<dyn std::error::Error>> {
        let rows = self.w()?.query(
            "SELECT lichessid, lichessusername, status, \
             registrantcomment, tdcomment, special FROM registrations WHERE lichessid = $1",
            &[&lichess_id],
        )?;
        Ok(rows.get(0).map(|row| Registration {
            lichess_id: row.get(0),
            lichess_username: row.get(1),
            status: row.get(2),
            registrant_comment: row.get(3),
            td_comment: row.get(4),
            special: row.get(5),
        }))
    }

    fn all_registrations(&self) -> Result<Vec<Registration>, Box<dyn std::error::Error>> {
        let rows = self.w()?.query(
            "SELECT lichessid, lichessusername, status, registrantcomment, \
             tdcomment, special FROM registrations",
            &[],
        )?;
        let mut registrations: Vec<Registration> = vec![];
        for row in rows {
            registrations.push(Registration {
                lichess_id: row.get(0),
                lichess_username: row.get(1),
                status: row.get(2),
                registrant_comment: row.get(3),
                td_comment: row.get(4),
                special: row.get(5),
            });
        }
        Ok(registrations)
    }

    fn approve_registration(
        &self,
        lichess_id: &str,
        td_comment: &str,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        set_status(self, lichess_id, td_comment, STATUS_APPROVED)
    }

    fn reject_registration(
        &self,
        lichess_id: &str,
        td_comment: &str,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        set_status(self, lichess_id, td_comment, STATUS_REJECTED)
    }

    fn withdraw_registration(&self, lichess_id: &str) -> Result<u64, Box<dyn std::error::Error>> {
        Ok(self.w()?.execute(
            "DELETE FROM registrations WHERE lichessid = $1",
            &[&lichess_id],
        )?)
    }
}
