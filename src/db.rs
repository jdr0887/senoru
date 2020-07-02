use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};

pub type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

pub fn create_db_connection_pool() -> DbPool {
    let project_dir = dirs::home_dir().unwrap().join(".senoru");
    if !project_dir.as_path().exists() {
        std::fs::create_dir_all(&project_dir).ok();
    }
    let connspec = project_dir.clone().join("senoru.db");
    let manager = ConnectionManager::<SqliteConnection>::new(connspec.to_string_lossy());
    let pool = r2d2::Pool::builder().max_size(1).build(manager).expect("Failed to create pool.");
    pool
}
