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
use humantime::format_duration;
use log::Level;
use std::error::Error;
use std::str::FromStr;
use std::time::Instant;
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
    let start = Instant::now();
    let options = Options::from_args();
    let log_level = Level::from_str(options.log_level.as_str()).expect("Invalid log level");
    simple_logger::init_with_level(log_level).unwrap();
    debug!("{:?}", options);

    db::init_db()?;

    let application_window: gtk::Application = gtk::Application::new(Some("com.kiluet.senoru"), Default::default()).expect("initialize failed");
    application_window.connect_startup(move |application| {
        let dialog = gtk::DialogBuilder::new().build();

        gui::launch(application).expect("failed to launch the gui");
    });
    application_window.connect_activate(|_| {});
    application_window.run(&[]);

    info!("Duration: {}", format_duration(start.elapsed()).to_string());
    Ok(())
}
