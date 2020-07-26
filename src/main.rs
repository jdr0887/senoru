#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;
extern crate gdk;
extern crate gio;
extern crate glib;
extern crate gtk;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate magic_crypt;
extern crate base64;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::DialogExt;
use log::Level;
use passwords::analyzer;
use passwords::scorer;
use std::env;
use std::error::Error;
use std::path;
use std::str::FromStr;
use structopt::StructOpt;

mod db;
mod gui;
mod item_actions;
mod models;
mod schema;

#[derive(StructOpt, Debug)]
#[structopt(name = "senoru", about = "senoru")]
struct Options {
    #[structopt(short = "l", long = "log_level", long_help = "log level", default_value = "info")]
    log_level: String,

    #[structopt(short = "f", long = "database_file", long_help = "database file", parse(from_os_str))]
    database: Option<path::PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::from_args();
    let log_level = Level::from_str(options.log_level.as_str()).expect("Invalid log level");
    simple_logger::init_with_level(log_level).unwrap();
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

    let application: gtk::Application = gtk::Application::new(Some("com.kiluet.senoru"), Default::default()).expect("initialize failed");

    application.connect_activate(move |app| {
        start_ui(&app);
    });
    application.run(&[]);

    Ok(())
}

fn start_ui(app: &gtk::Application) {
    let builder: gtk::Builder = gtk::Builder::from_string(include_str!("senoru.glade"));
    let passphrase_dialog: gtk::Dialog = builder.get_object("passphrase_dialog").unwrap();
    let passphrase_dialog_ok_button: gtk::Button = builder.get_object("passphrase_dialog_ok_button").unwrap();
    let passphrase_dialog_cancel_button: gtk::Button = builder.get_object("passphrase_dialog_cancel_button").unwrap();
    let passphrase_dialog_entry: gtk::Entry = builder.get_object("passphrase_dialog_entry").unwrap();
    let passphrase_dialog_quality_score_label: gtk::Label = builder.get_object("passphrase_dialog_quality_score_label").unwrap();

    db::init_db().expect("failed to initialize the db");

    passphrase_dialog_entry.connect_key_release_event(
        glib::clone!(@weak passphrase_dialog_quality_score_label => @default-return Inhibit(false), move | entry, key | {
            let passphrase = entry.get_buffer().get_text();
            let score = scorer::score(&analyzer::analyze(&passphrase));
            passphrase_dialog_quality_score_label.set_label(format!("{}/100", score as i32).as_str());
            Inhibit(false)
        }),
    );

    passphrase_dialog_entry.connect_activate(
        glib::clone!(@weak app, @weak builder, @weak passphrase_dialog, @weak passphrase_dialog_entry => move |_| {
            passphrase_dialog_ok_button_clicked(&app, &builder, &passphrase_dialog, &passphrase_dialog_entry);
        }),
    );
    passphrase_dialog_ok_button.connect_clicked(
        glib::clone!(@weak app, @weak builder, @weak passphrase_dialog, @weak passphrase_dialog_entry => move |_| {
            passphrase_dialog_ok_button_clicked(&app, &builder, &passphrase_dialog, &passphrase_dialog_entry);
        }),
    );
    passphrase_dialog_cancel_button.connect_clicked(|_| {
        std::process::exit(0);
    });
    passphrase_dialog.run();
    passphrase_dialog.close();
}

fn passphrase_dialog_ok_button_clicked(app: &gtk::Application, builder: &gtk::Builder, passphrase_dialog: &gtk::Dialog, passphrase_dialog_entry: &gtk::Entry) {
    let items = item_actions::find_all(Some(1i64)).expect("failed to get items from db");
    let mc = new_magic_crypt!(passphrase_dialog_entry.get_buffer().get_text(), 256);
    let first_item = items.first();
    match first_item {
        Some(item) => match item.clone().decrypt_contents(&mc) {
            Ok(_) => {
                gui::launch(&app, &builder, &mc).expect("failed to launch the gui");
                passphrase_dialog.close();
            }
            Err(e) => {
                warn!("error message: {}", e.to_string().as_str());
                let error_dialog: gtk::MessageDialog = builder.get_object("error_dialog").unwrap();
                error_dialog.set_property_text("Invalid passphrase".into());
                error_dialog.run();
                error_dialog.close();
            }
        },
        None => {
            gui::launch(&app, &builder, &mc).expect("failed to launch the gui");
            passphrase_dialog.close();
        }
    }
}
