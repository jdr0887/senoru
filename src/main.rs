#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
extern crate gdk;
extern crate gio;
extern crate glib;
extern crate gtk;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate magic_crypt;

use std::env;
use std::error;
use std::path;
use std::sync::{Arc, Mutex};
use clap::Parser;
use gio::prelude::*;
use gtk::prelude::*;
use passwords::analyzer;
use passwords::scorer;

mod db;
mod gui;
mod item_actions;
mod models;
mod schema;

pub struct AppCore {
    pub magic_crypt: Arc<Mutex<Option<magic_crypt::MagicCrypt256>>>,
}

lazy_static! {
    static ref APP_CORE: AppCore = AppCore {
        magic_crypt: Arc::new(Mutex::new(None))
    };
}

#[derive(Parser, PartialEq, Debug)]
#[clap(author, version, about, long_about = None)]
struct Options {
    #[clap(short, long)]
    database: Option<path::PathBuf>,
}

fn main() -> Result<(), Box<dyn error::Error>> {
    env_logger::init();

    let options = Options::parse();
    debug!("{:?}", options);

    let db_path = match options.database {
        Some(p) => p,
        None => {
            let project_dir = dirs::home_dir().unwrap().join(".senoru");
            if !project_dir.as_path().exists() {
                std::fs::create_dir_all(&project_dir).ok();
            }
            project_dir.clone().join("senoru.db")
        }
    };
    env::set_var("SENORU_DB", db_path.as_os_str());

    let application = gtk::Application::builder().application_id("com.kiluet.senoru").build();

    application.connect_activate(move |app| {
        start_ui(&app);
    });
    let args: Vec<String> = vec![];
    application.run_with_args(&args);

    Ok(())
}

fn start_ui(app: &gtk::Application) {
    let builder: gtk::Builder = gtk::Builder::from_string(include_str!("senoru.glade"));
    let key_dialog: gtk::Dialog = builder.object("key_dialog").unwrap();
    let key_dialog_ok_button: gtk::Button = builder.object("key_dialog_ok_button").unwrap();
    let key_dialog_cancel_button: gtk::Button = builder.object("key_dialog_cancel_button").unwrap();
    let key_dialog_entry: gtk::Entry = builder.object("key_dialog_entry").unwrap();
    let key_dialog_quality_score_label: gtk::Label = builder.object("key_dialog_quality_score_label").unwrap();

    db::init_db().expect("failed to initialize the db");

    key_dialog_entry.connect_key_release_event(gtk::glib::clone!(@weak key_dialog_quality_score_label => @default-return Inhibit(false), move | entry, _ | {
        let key = entry.buffer().text();
        let score = scorer::score(&analyzer::analyze(&key));
        key_dialog_quality_score_label.set_label(format!("{}/100", score as i32).as_str());
        Inhibit(false)
    }));

    key_dialog_entry.connect_activate(glib::clone!(@weak app, @weak builder, @weak key_dialog, @weak key_dialog_entry => move |_| {
        key_dialog_ok_button_clicked(&app, &builder, &key_dialog, &key_dialog_entry);
    }));
    key_dialog_ok_button.connect_clicked(glib::clone!(@weak app, @weak builder, @weak key_dialog, @weak key_dialog_entry => move |_| {
        key_dialog_ok_button_clicked(&app, &builder, &key_dialog, &key_dialog_entry);
    }));
    key_dialog_cancel_button.connect_clicked(|_| {
        std::process::exit(0);
    });
    key_dialog.run();
    key_dialog.close();
}

fn key_dialog_ok_button_clicked(app: &gtk::Application, builder: &gtk::Builder, key_dialog: &gtk::Dialog, key_dialog_entry: &gtk::Entry) {
    let items = item_actions::find_all(Some(1i64)).expect("failed to get items from db");
    let magic_crypt = new_magic_crypt!(key_dialog_entry.buffer().text(), 256);
    // let app_core = AppCore::new(app.clone(), builder.clone(), magic_crypt.clone());
    let mut mc = APP_CORE.magic_crypt.lock().unwrap();
    *mc = Some(magic_crypt.clone());
    let first_item = items.first();
    match first_item {
        Some(item) => match item.decrypt_contents(&magic_crypt) {
            Ok(_) => {
                gui::launch(app, builder).expect("failed to launch the gui");
                key_dialog.close();
            }
            Err(e) => {
                warn!("error message: {}", e.to_string().as_str());
                let error_dialog: gtk::MessageDialog = builder.object("error_dialog").unwrap();
                error_dialog.set_text("Invalid key".into());
                error_dialog.run();
                error_dialog.close();
            }
        },
        None => {
            gui::launch(app, builder).expect("failed to launch the gui");
            key_dialog.close();
        }
    }
}
