use diesel::debug_query;
use diesel::prelude::*;
use diesel::sqlite::Sqlite;

use crate::db;
use crate::models;
use crate::schema::items;

pub fn find_all(limit: Option<i64>) -> Result<Vec<models::Item>, diesel::result::Error> {
    let conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let results = match limit {
        Some(l) => items::table
            .order(items::dsl::title)
            .limit(l)
            .load::<models::Item>(&conn)
            .expect("failed to find all"),
        None => items::table.order(items::dsl::title).load::<models::Item>(&conn).expect("failed to find all"),
    };
    Ok(results)
}

#[allow(dead_code)]
pub fn find_by_id(gid: i32) -> Result<Option<models::Item>, diesel::result::Error> {
    let conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let item = items::table.filter(items::dsl::id.eq(gid));
    debug!("{}", debug_query::<Sqlite, _>(&item).to_string());
    let results = item.first::<models::Item>(&conn).optional()?;
    Ok(results)
}

pub fn find_by_title(title: &String) -> Result<Option<models::Item>, diesel::result::Error> {
    let conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let item = items::table.filter(items::dsl::title.eq(title));
    debug!("{}", debug_query::<Sqlite, _>(&item).to_string());
    let results = item.first::<models::Item>(&conn).optional()?;
    Ok(results)
}

pub fn insert(new_item: &models::NewItem) -> Result<bool, diesel::result::Error> {
    let conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let insert = diesel::insert_into(items::table).values(new_item);
    debug!("{}", debug_query::<Sqlite, _>(&insert).to_string());
    let num_inserted = insert.execute(&conn)?;
    debug!("num_inserted: {}", num_inserted);
    Ok(num_inserted == 1)
}

pub fn delete(gid: &i32) -> Result<bool, diesel::result::Error> {
    let conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let delete = diesel::delete(items::table.filter(items::dsl::id.eq(gid)));
    debug!("{}", debug_query::<Sqlite, _>(&delete).to_string());
    let num_deleted = delete.execute(&conn)?;
    debug!("num_deleted: {}", num_deleted);
    Ok(num_deleted == 1)
}

pub fn update(item: &models::Item) -> Result<bool, diesel::result::Error> {
    let conn = db::DB_POOL.get().expect("failed to get db connection from pool");
    let update = diesel::update(items::table.filter(items::dsl::id.eq(item.id))).set(item);
    debug!("{}", debug_query::<Sqlite, _>(&update).to_string());
    let num_updated = update.execute(&conn)?;
    debug!("num_updated: {}", num_updated);
    Ok(num_updated == 1)
}
