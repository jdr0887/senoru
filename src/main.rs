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
use humantime::format_duration;
use log::Level;
use std::error::Error;
use std::str::FromStr;
use std::time::Instant;
use structopt::StructOpt;

mod db;
mod item_actions;
mod models;
mod schema;

embed_migrations!();

#[derive(StructOpt, Debug)]
#[structopt(name = "senoru", about = "senoru")]
struct Options {
    #[structopt(short = "l", long = "log_level", long_help = "log level", default_value = "info")]
    log_level: String,
}

lazy_static! {
    pub static ref DB_POOL: db::DbPool = db::create_db_connection_pool();
    pub static ref BUFFER_SIZE: usize = 2_usize.pow(12);
}

fn init_db() -> Result<(), Box<dyn Error>> {
    let conn = DB_POOL.get().expect("failed to get db connection from pool");
    embedded_migrations::run(&conn)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    let options = Options::from_args();
    let log_level = Level::from_str(options.log_level.as_str()).expect("Invalid log level");
    simple_logger::init_with_level(log_level).unwrap();
    debug!("{:?}", options);

    init_db()?;

    let application_window: gtk::Application = gtk::Application::new(Some("com.kiluet.senoru"), Default::default()).expect("initialize failed");
    application_window.connect_startup(move |application| {
        let dialog = gtk::DialogBuilder::new().build();

        launch(application).expect("failed to launch the gui");
    });
    application_window.connect_activate(|_| {});
    application_window.run(&[]);

    info!("Duration: {}", format_duration(start.elapsed()).to_string());
    Ok(())
}

fn launch(application: &gtk::Application) -> Result<(), Box<dyn Error>> {
    let builder: gtk::Builder = gtk::Builder::new_from_string(include_str!("senoru.glade"));
    let main_window: gtk::Window = builder.get_object("main_window").unwrap();
    let about_menu_item: gtk::MenuItem = builder.get_object("about_menu_item").unwrap();
    let import_menu_item: gtk::MenuItem = builder.get_object("import_menu_item").unwrap();
    let new_menu_item: gtk::MenuItem = builder.get_object("new_menu_item").unwrap();
    let export_menu_item: gtk::MenuItem = builder.get_object("export_menu_item").unwrap();
    let quit_menu_item: gtk::MenuItem = builder.get_object("quit_menu_item").unwrap();
    let tree_view: gtk::TreeView = builder.get_object("tree_view").unwrap();
    let text_view: gtk::TextView = builder.get_object("text_view").unwrap();

    let store = create_store()?;
    tree_view.set_model(Some(&store));

    // new_menu_item.connect_activate(glib::clone!(@weak main_window => move |_| {
    // }));

    import_menu_item.connect_activate(glib::clone!(@weak main_window => move |_| {
        let file_chooser_dialog = gtk::FileChooserDialogBuilder::new()
            .title("Choose a file to import")
            .show_hidden(true)
            .select_multiple(true)
            .parent(&main_window)
            .transient_for(&main_window)
            .action(gtk::FileChooserAction::Open)
            .build();

        file_chooser_dialog.add_buttons(&[
            ("Open", gtk::ResponseType::Ok),
            ("Cancel", gtk::ResponseType::Cancel)
        ]);

        if file_chooser_dialog.run() == gtk::ResponseType::Ok {

            let files = file_chooser_dialog.get_filenames();
            files.iter().for_each(|z| info!("file: {}", z.to_string_lossy()));
            for path in files.iter() {
                let conn = DB_POOL.get().expect("failed to get db connection from pool");
                let item_title: String = path.file_name().unwrap().to_os_string().into_string().unwrap();
                let mut new_item = models::NewItem::new(item_title.clone());
                let contents = std::fs::read_to_string(path.as_path()).unwrap();
                new_item.contents = Some(contents);
                match item_actions::insert(&new_item, &conn) {
                    Ok(_) => {
                        let value = glib::value::Value::from(&item_title);
                        store.set_value(&store.append(), 0u32, &value);
                    },
                    Err(e) => warn!("{}", e)
                }
            }
        }

        file_chooser_dialog.destroy();
    }));

    about_menu_item.connect_activate(glib::clone!(@weak main_window => move |_| {

        let about_dialog = gtk::AboutDialogBuilder::new()
            .title("About Secure Notepad in Rust")
            .version(env!("CARGO_PKG_VERSION"))
            .authors(vec![env!("CARGO_PKG_AUTHORS").to_string()])
            .website_label("Secure Notepad in Rust")
            .website("http://github.com/jdr0887/senoru")
            .transient_for(&main_window)
            .build();

        about_dialog.run();
        about_dialog.destroy();
    }));

    let column = create_column()?;
    tree_view.append_column(&column);

    let left_selection = tree_view.get_selection();
    left_selection.connect_changed(glib::clone!(@weak text_view => move |tree_selection| {
        let (left_model, iter) = tree_selection.get_selected().expect("Couldn't get selected");
        let selected_title = left_model.get_value(&iter, 0).get::<String>().expect("failed to get selected title");
        let text_view_buffer = text_view.get_buffer().expect("Couldn't get buffer");
        match selected_title {
            Some(title) => {
                let conn = DB_POOL.get().expect("couldn't get db connection from pool");
                let item = item_actions::find_by_title(title, &conn).expect("failed to find Item by title");
                match item {
                    Some(i) => {
                        text_view_buffer.set_text(&i.contents.unwrap().as_str());
                    },
                    None => text_view_buffer.set_text(&"")
                }
            },
            _ => text_view_buffer.set_text(&"")
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

fn create_store() -> Result<gtk::ListStore, Box<dyn Error>> {
    let store = gtk::ListStore::new(&[glib::Type::String]);
    let conn = DB_POOL.get().expect("couldn't get db connection from pool");
    let items = item_actions::find_all(&conn).expect("failed to get Items");
    for item in items.iter() {
        debug!("item: {:?}", item);
        let value = glib::value::Value::from(&item.title);
        store.set_value(&store.append(), 0u32, &value);
    }
    Ok(store)
}
