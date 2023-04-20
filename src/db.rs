use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use std::env;
use std::error::Error;
use std::path;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};

pub type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

lazy_static! {
    pub static ref DB_POOL: DbPool = create_db_connection_pool();
}

pub fn create_db_connection_pool() -> DbPool {
    let senoru_db = env::var("SENORU_DB").unwrap();
    debug!("using SENORU_DB: {}", senoru_db);
    let connspec = path::PathBuf::new().join(senoru_db);
    let manager = ConnectionManager::<SqliteConnection>::new(connspec.to_string_lossy());
    let pool = r2d2::Pool::builder().max_size(1).build(manager).expect("Failed to create pool.");
    pool
}

pub fn init_db() -> Result<(), Box<dyn Error>> {
    let mut conn = DB_POOL.get().expect("failed to get db connection from pool");
    conn.run_pending_migrations(MIGRATIONS).unwrap();
    Ok(())
}

