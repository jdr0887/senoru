use diesel::debug_query;
use diesel::prelude::*;
use diesel::sqlite::Sqlite;

use crate::models;
use crate::schema::items;

pub fn find_all(conn: &SqliteConnection) -> Result<Vec<models::Item>, diesel::result::Error> {
    let results = items::table.load::<models::Item>(conn).expect("failed to find all");
    Ok(results)
}

pub fn find_by_id(gid: i32, conn: &SqliteConnection) -> Result<Option<models::Item>, diesel::result::Error> {
    let item = items::table.filter(items::dsl::id.eq(gid));
    debug!("{}", debug_query::<Sqlite, _>(&item).to_string());
    let results = item.first::<models::Item>(conn).optional()?;
    Ok(results)
}

pub fn find_by_title(title: String, conn: &SqliteConnection) -> Result<Option<models::Item>, diesel::result::Error> {
    let item = items::table.filter(items::dsl::title.eq(title));
    debug!("{}", debug_query::<Sqlite, _>(&item).to_string());
    let results = item.first::<models::Item>(conn).optional()?;
    Ok(results)
}

pub fn insert(new_item: &models::NewItem, conn: &SqliteConnection) -> Result<bool, diesel::result::Error> {
    let insert = diesel::insert_into(items::table).values(new_item);
    debug!("{}", debug_query::<Sqlite, _>(&insert).to_string());
    let num_inserted = insert.execute(conn)?;
    debug!("num_inserted: {}", num_inserted);
    Ok(num_inserted == 1)
}

pub fn delete(gid: i32, conn: &SqliteConnection) -> Result<bool, diesel::result::Error> {
    let delete = diesel::delete(items::table.filter(items::dsl::id.eq(gid)));
    debug!("{}", debug_query::<Sqlite, _>(&delete).to_string());
    let num_deleted = delete.execute(conn)?;
    debug!("num_deleted: {}", num_deleted);
    Ok(num_deleted == 1)
}

pub fn update(gift: &models::Item, conn: &SqliteConnection) -> Result<bool, diesel::result::Error> {
    let update = diesel::update(items::table.filter(items::dsl::id.eq(gift.id))).set(gift);
    debug!("{}", debug_query::<Sqlite, _>(&update).to_string());
    let num_updated = update.execute(conn)?;
    debug!("num_updated: {}", num_updated);
    Ok(num_updated == 1)
}
