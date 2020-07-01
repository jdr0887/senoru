#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;
extern crate gio;
extern crate glib;
extern crate gtk;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate diesel_migrations;

use gio::prelude::*;
use gtk::prelude::*;

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use humantime::format_duration;
use log::Level;
use std::error::Error;
use std::str::FromStr;
use std::time::Instant;
use structopt::StructOpt;

mod models;
mod schema;

pub type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

embed_migrations!();

#[derive(StructOpt, Debug)]
#[structopt(name = "senoru", about = "senoru")]
struct Options {
    #[structopt(short = "l", long = "log_level", long_help = "log level", default_value = "info")]
    log_level: String,
}

fn create_db_connection_pool() -> DbPool {
    let securu_dir = dirs::home_dir().unwrap().join(".senoru");
    if !securu_dir.as_path().exists() {
        std::fs::create_dir_all(&securu_dir).ok();
    }
    let connspec = securu_dir.clone().join("senoru.db");
    let manager = ConnectionManager::<SqliteConnection>::new(connspec.to_string_lossy());
    let pool = r2d2::Pool::builder().max_size(1).build(manager).expect("Failed to create pool.");
    pool
}

lazy_static! {
    pub static ref DB_POOL: DbPool = create_db_connection_pool();
}

fn main() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    let options = Options::from_args();
    let log_level = Level::from_str(options.log_level.as_str()).expect("Invalid log level");
    simple_logger::init_with_level(log_level).unwrap();
    debug!("{:?}", options);

    init_db()?;

    let application_window: gtk::Application = gtk::Application::new(Some("com.kiluet.securu"), Default::default()).expect("initialize failed");
    application_window.connect_startup(move |application| {
        launch(application).expect("failed to launch the gui");
    });
    application_window.connect_activate(|_| {});
    application_window.run(&[]);

    info!("Duration: {}", format_duration(start.elapsed()).to_string());
    Ok(())
}

fn init_db() -> Result<(), Box<dyn Error>> {
    let conn = DB_POOL.get()?;
    embedded_migrations::run(&conn)?;
    Ok(())
}

fn launch(application: &gtk::Application) -> Result<(), Box<dyn Error>> {
    let builder: gtk::Builder = gtk::Builder::new_from_string(include_str!("senoru.glade"));

    let main_window: gtk::Window = builder.get_object("main_window").unwrap();
    let quit_menu_item: gtk::MenuItem = builder.get_object("quit_menu_item").unwrap();
    let tree_view: gtk::TreeView = builder.get_object("tree_view").unwrap();
    let text_view: gtk::TextView = builder.get_object("text_view").unwrap();

    let column = create_column()?;
    tree_view.append_column(&column);

    let model = create_model()?;
    tree_view.set_model(Some(&model));

    let left_selection = tree_view.get_selection();
    left_selection.connect_changed(glib::clone!(@weak text_view => move |tree_selection| {
        let (left_model, iter) = tree_selection.get_selected().expect("Couldn't get selected");
        let selected_title_option = left_model.get_value(&iter, 0).get::<String>().expect("failed to get selected title");
        match selected_title_option {
            Some(i) => {
                let conn = DB_POOL.get().expect("couldn't get db connection from pool");
                let db_items = schema::items::table.filter(schema::items::dsl::title.eq(i));
                let item = db_items.first::<models::Item>(&conn).expect("error");
                let text_view_buffer = text_view.get_buffer().expect("Couldn't get buffer");
                match item.contents {
                    Some(c) =>  text_view_buffer.set_text(&c.as_str()),
                    _ =>  text_view_buffer.set_text(&""),
                };
            },
            _ => {}
         }
    }));

    quit_menu_item.connect_activate(glib::clone!(@weak main_window => move |_| {
        main_window.destroy();
    }));

    main_window.set_application(Some(application));
    // main_window.connect_delete_event(clone!(main_window => move |_, _| {
    //     main_window.destroy();
    //     Inhibit(false)
    // }));

    main_window.show_all();
    Ok(())
}

fn create_column() -> Result<gtk::TreeViewColumn, Box<dyn Error>> {
    let renderer = gtk::CellRendererText::new();
    let column = gtk::TreeViewColumn::new();
    column.pack_start(&renderer, true);
    column.set_title("Title");
    column.add_attribute(&renderer, "text", 0i32);
    column.set_sort_column_id(0i32);
    Ok(column)
}

fn create_model() -> Result<gtk::ListStore, Box<dyn Error>> {
    let conn = DB_POOL.get().expect("couldn't get db connection from pool");
    let items: Vec<models::Item> = schema::items::table.load::<models::Item>(&conn).expect("failed to find all");

    let store = gtk::ListStore::new(&[glib::Type::String]);
    for item in items.iter() {
        debug!("item: {:?}", item);
        let value = glib::value::Value::from(&item.title);
        store.set_value(&store.append(), 0u32, &value);
    }

    Ok(store)
}
