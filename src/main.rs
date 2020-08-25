#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate magic_crypt;
extern crate base64;

use log::Level;
use orbtk::prelude::*;
use passwords::analyzer;
use passwords::scorer;
use std::env;
use std::error::Error;
use std::path;
use std::str::FromStr;
use structopt::StructOpt;

mod db;
mod item_actions;
mod models;
mod schema;

const KEY_INPUT_ID: &str = "key_input";

enum LoginAction {
    Authenticate,
    ShowPopup,
    ClosePopup,
}

#[derive(Default, AsAny)]
struct LoginFormState {
    authenticated: bool,
    action: Option<LoginAction>,
    key_input: Entity,
    show_popup: bool,
    popup: Option<Entity>,
}

impl LoginFormState {
    fn authenticate(&mut self) {
        if !self.show_popup {
            self.action = Some(LoginAction::Authenticate);
        }
    }

    fn show_popup(&mut self) {
        if !self.show_popup {
            self.action = Some(LoginAction::ShowPopup);
        }
    }

    fn close_popup(&mut self) {
        if self.show_popup {
            self.action = Some(LoginAction::ClosePopup);
        }
    }

    // creates a popup based on the authenticated field and returns its entity
    fn create_popup(&self, target: Entity, build_context: &mut BuildContext) -> Entity {
        let (msg, text_color) = match self.authenticated {
            true => ("Login success!", "#4CA64C"),
            false => ("Login failed!", "#FF3232"),
        };

        Popup::new()
            .style("popup")
            .target(target)
            .open(true)
            .width(175.0)
            .height(125.0)
            .h_align("center")
            .v_align("center")
            .child(
                Container::new()
                    .border_radius(3.0)
                    .border_width(2.0)
                    .padding(8.0)
                    .child(
                        TextBlock::new()
                            .font_size(18.0)
                            .foreground(text_color)
                            .h_align("center")
                            .v_align("top")
                            .text(msg)
                            .build(build_context),
                    )
                    .child(
                        Button::new()
                            .h_align("center")
                            .v_align("center")
                            .text("OK")
                            // Send a ClosePopup action to LoginFormState when button is clicked
                            .on_click(move |states, _point| -> bool {
                                states.get_mut::<LoginFormState>(target).close_popup();
                                true
                            })
                            .build(build_context),
                    )
                    .build(build_context),
            )
            .build(build_context)
    }
}

impl State for LoginFormState {
    fn init(&mut self, _: &mut Registry, ctx: &mut Context) {
        self.authenticated = false;
        self.key_input = ctx.entity_of_child(KEY_INPUT_ID).expect("Invalid Key");
        self.show_popup = false;
        self.popup = None;
    }

    fn update(&mut self, reg: &mut Registry, ctx: &mut Context) {
        if let Some(action) = &self.action {
            match action {
                LoginAction::Authenticate => {
                    let key = ctx.get_widget(self.key_input).get::<String16>("text").as_string();

                    let items = item_actions::find_all(Some(1i64)).expect("failed to get items from db");
                    let mc = new_magic_crypt!(key, 256);
                    let first_item = items.first();
                    match first_item {
                        Some(item) => match item.clone().decrypt_contents(&mc) {
                            Ok(_) => {
                                self.authenticated = true;
                            }
                            Err(e) => {
                                warn!("error message: {}", e.to_string().as_str());
                                self.authenticated = false;
                            }
                        },
                        None => {
                            self.authenticated = true;
                        }
                    }

                    self.show_popup();
                    self.update(reg, ctx);
                }
                // creates a popup then attach it to the overlay
                LoginAction::ShowPopup => {
                    let current_entity = ctx.entity;
                    let build_context = &mut ctx.build_context();
                    let popup = self.create_popup(current_entity, build_context);
                    build_context.append_child(current_entity, popup);
                    self.show_popup = true;
                    self.popup = Some(popup);
                }
                // delete popup from widget tree.
                LoginAction::ClosePopup => {
                    if let Some(popup) = self.popup {
                        self.show_popup = false;
                        ctx.remove_child(popup);
                    }
                }
            }

            self.action = None;
        }
    }
}

widget!(LoginForm<LoginFormState>);

impl Template for LoginForm {
    fn template(self, id: Entity, ctx: &mut BuildContext) -> Self {
        self.name("LoginForm").child(
            Grid::new()
                .columns(Columns::create().push(64.0).push(64.0))
                .rows(Rows::create().push(48.0).push(48.0).push(48.0).push(48.0))
                .v_align("start")
                .h_align("center")
                .child(
                    TextBlock::new()
                        .text("Welcome to SENORU")
                        .font_size(18.0)
                        .v_align("center")
                        .h_align("center")
                        .attach(Grid::column(0))
                        .attach(Grid::row(0))
                        .attach(Grid::column_span(4))
                        .build(ctx),
                )
                .child(
                    TextBlock::new()
                        .text("Key:")
                        .v_align("center")
                        .h_align("center")
                        .attach(Grid::column(0))
                        .attach(Grid::row(1))
                        .build(ctx),
                )
                .child(
                    PasswordBox::new()
                        .id(KEY_INPUT_ID)
                        .water_mark("Key")
                        .v_align("center")
                        .h_align("left")
                        .attach(Grid::column(1))
                        .attach(Grid::row(1))
                        .max_width(160.0)
                        .build(ctx),
                )
                .child(
                    Button::new()
                        .text("Login")
                        .v_align("center")
                        .h_align("end")
                        .attach(Grid::column(1))
                        .attach(Grid::row(2))
                        .on_click(move |states, _| -> bool {
                            states.get_mut::<LoginFormState>(id).authenticate();
                            false
                        })
                        .build(ctx),
                )
                .build(ctx),
        )
    }
}

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

    Application::new()
        .window(|ctx| {
            Window::new()
                .title("SENORU - Secure Notepad in Rust")
                .position((200.0, 200.0))
                .size(400.0, 180.0)
                .resizeable(true)
                .child(LoginForm::new().build(ctx))
                .build(ctx)
        })
        .run();
    Ok(())
}
