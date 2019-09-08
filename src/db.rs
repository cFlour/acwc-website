use postgres::NoTls;
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

#[derive(Serialize)]
pub struct QualificationEntrant {
    pub seed: i32,
    pub lichess_id: String,
    pub lichess_username: String,
    pub latest_rating: i32,
    pub latest_rating_url: String,
    pub highest_rating: i32,
    pub highest_rating_url: String,
    pub seeding_rating: f64,
}

pub fn connect(connection_options: &str) -> Result<AcwcDbClient, Box<dyn std::error::Error>> {
    let manager = PostgresConnectionManager::new(connection_options.parse()?, NoTls);
    let pool = Pool::new(manager)?;
    Ok(AcwcDbClient(pool))
}

pub struct AcwcDbClient(DbPool);

impl AcwcDbClient {
    fn w(&self) -> Result<DbConnection, Box<dyn std::error::Error>> {
        Ok(self.0.get()?)
    }

    pub fn insert_registration(
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

    pub fn find_registration(
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

    pub fn all_registrations(&self) -> Result<Vec<Registration>, Box<dyn std::error::Error>> {
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

    fn set_status(
        &self,
        lichess_id: &str,
        td_comment: &str,
        status: i32,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        Ok(self.w()?.execute(
            "UPDATE registrations SET status = $1, tdcomment = $2 WHERE lichessid = $3",
            &[&status, &td_comment, &lichess_id],
        )?)
    }

    pub fn approve_registration(
        &self,
        lichess_id: &str,
        td_comment: &str,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        self.set_status(lichess_id, td_comment, STATUS_APPROVED)
    }

    pub fn reject_registration(
        &self,
        lichess_id: &str,
        td_comment: &str,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        self.set_status(lichess_id, td_comment, STATUS_REJECTED)
    }

    pub fn withdraw_registration(
        &self,
        lichess_id: &str,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        Ok(self.w()?.execute(
            "DELETE FROM registrations WHERE lichessid = $1",
            &[&lichess_id],
        )?)
    }

    pub fn qualification_entrants(
        &self,
    ) -> Result<Vec<QualificationEntrant>, Box<dyn std::error::Error>> {
        let rows = self.w()?.query(
            "SELECT lichessid, lichessusername, latestrating, \
             latestratingurl, highestrating, highestratingurl FROM qualification \
             ORDER BY (latestrating+highestrating) DESC",
            &[],
        )?;
        let mut entrants: Vec<QualificationEntrant> = vec![];
        for (i, row) in rows.iter().enumerate() {
            let latest_rating = row.get(2);
            let highest_rating = row.get(4);
            let seeding_rating = (latest_rating + highest_rating) as f64 / 2.0;
            entrants.push(QualificationEntrant {
                seed: i as i32 + 1,
                lichess_id: row.get(0),
                lichess_username: row.get(1),
                latest_rating,
                latest_rating_url: row.get(3),
                highest_rating,
                highest_rating_url: row.get(5),
                seeding_rating,
            });
        }
        Ok(entrants)
    }
}
