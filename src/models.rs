use super::schema::*;
use chrono::prelude::*;
use diesel::*;
use magic_crypt::MagicCrypt256;
use magic_crypt::MagicCryptTrait;
use serde_derive::{Deserialize, Serialize};
use std::error::Error;

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable, AsChangeset, Associations)]
#[changeset_options(treat_none_as_null = "true")]
#[table_name = "items"]
pub struct Item {
    pub id: i32,
    pub title: String,
    pub contents: Option<String>,
    pub date_added: NaiveDateTime,
    pub date_last_modified: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Queryable, Insertable, AsChangeset, Associations)]
#[changeset_options(treat_none_as_null = "true")]
#[table_name = "items"]
pub struct NewItem {
    pub title: String,
    pub contents: Option<String>,
    pub date_added: NaiveDateTime,
    pub date_last_modified: NaiveDateTime,
}

impl NewItem {
    pub fn new(title: String) -> NewItem {
        NewItem {
            title,
            contents: None,
            date_added: Utc::now().naive_utc(),
            date_last_modified: Utc::now().naive_utc(),
        }
    }
}

impl Item {
    pub fn decrypt_contents(&self, mc: &MagicCrypt256) -> Result<String, Box<dyn Error>> {
        let contents = mc.decrypt_base64_to_string(self.contents.as_ref().unwrap())?;
        Ok(contents)
    }
}
