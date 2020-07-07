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
use std::error::Error;
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
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::from_args();
    let log_level = Level::from_str(options.log_level.as_str()).expect("Invalid log level");
    simple_logger::init_with_level(log_level).unwrap();
    debug!("{:?}", options);

    let application: gtk::Application = gtk::Application::new(Some("com.kiluet.senoru"), Default::default()).expect("initialize failed");

    application.connect_activate(move |app| {
        start_ui(&app);
    });
    application.run(&[]);

    Ok(())
}

fn start_ui(app: &gtk::Application) {
    let builder: gtk::Builder = gtk::Builder::from_string(include_str!("senoru.glade"));
    let main_window: gtk::Window = builder.get_object("main_window").unwrap();
    let dialog: gtk::Dialog = builder.get_object("passphrase_dialog").unwrap();
    let passphrase_dialog_entry: gtk::Entry = builder.get_object("passphrase_dialog_entry").unwrap();
    let passphrase_dialog_ok_button: gtk::Button = builder.get_object("passphrase_dialog_ok_button").unwrap();
    let passphrase_dialog_cancel_button: gtk::Button = builder.get_object("passphrase_dialog_cancel_button").unwrap();

    db::init_db().expect("failed to initialize the db");

    let dialog_clone = dialog.clone();
    passphrase_dialog_ok_button.connect_clicked(glib::clone!(@weak app => move |_| {
        passphrase_dialog_ok_button_clicked(&passphrase_dialog_entry, &dialog_clone, &app);
    }));
    passphrase_dialog_cancel_button.connect_clicked(glib::clone!(@weak main_window => move |_| {
        std::process::exit(0);
    }));
    dialog.run();
    dialog.close();
}

fn passphrase_dialog_ok_button_clicked(passphrase_dialog_entry: &gtk::Entry, dialog: &gtk::Dialog, app: &gtk::Application) {
    let items = item_actions::find_all(Some(1i64)).expect("failed to get items from db");
    let mc = new_magic_crypt!(passphrase_dialog_entry.get_buffer().get_text(), 256);
    let first_item = items.first();
    match first_item {
        Some(item) => match item.clone().decrypt_contents(&mc) {
            Ok(_) => {
                gui::launch(&app, &mc).expect("failed to launch the gui");
                dialog.close();
            }
            Err(e) => {
                warn!("error message: {}", e.to_string().as_str());
                let error_dialog = gtk::MessageDialogBuilder::new()
                    .title("Error")
                    .buttons(gtk::ButtonsType::Ok)
                    .message_type(gtk::MessageType::Error)
                    .modal(true)
                    .text("not valid passphrase")
                    .build();
                error_dialog.run();
                error_dialog.close();
            }
        },
        None => {
            gui::launch(&app, &mc).expect("failed to launch the gui");
            dialog.close();
        }
    }
}
