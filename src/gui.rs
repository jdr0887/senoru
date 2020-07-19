use crate::item_actions;
use crate::models;
use gtk::prelude::*;
use gtk::TreeViewExt;
use magic_crypt::MagicCrypt256;
use magic_crypt::MagicCryptTrait;
use std::error::Error;
use std::fs;
use std::io;
use std::io::prelude::*;

pub fn launch(application: &gtk::Application, builder: &gtk::Builder, mc: &MagicCrypt256) -> Result<(), Box<dyn Error>> {
    let main_window: gtk::Window = builder.get_object("main_window").unwrap();
    let about_menu_item: gtk::MenuItem = builder.get_object("about_menu_item").unwrap();
    let import_menu_item: gtk::MenuItem = builder.get_object("import_menu_item").unwrap();
    let new_menu_item: gtk::MenuItem = builder.get_object("new_menu_item").unwrap();
    let export_menu_item: gtk::MenuItem = builder.get_object("export_menu_item").unwrap();
    let remove_menu_item: gtk::MenuItem = gtk::MenuItemBuilder::new().label("Remove").build();
    let generate_password_menu_item: gtk::MenuItem = builder.get_object("generate_password_menu_item").unwrap();
    let quit_menu_item: gtk::MenuItem = builder.get_object("quit_menu_item").unwrap();
    let main_tree_view: gtk::TreeView = builder.get_object("main_tree_view").unwrap();
    let main_text_view: gtk::TextView = builder.get_object("main_text_view").unwrap();
    let about_dialog: gtk::AboutDialog = builder.get_object("about_dialog").unwrap();
    let generate_password_dialog: gtk::Dialog = builder.get_object("generate_password_dialog").unwrap();
    let generate_password_dialog_refresh_button: gtk::Button = builder.get_object("generate_password_dialog_refresh_button").unwrap();
    let generate_password_dialog_cancel_button: gtk::Button = builder.get_object("generate_password_dialog_cancel_button").unwrap();

    let store = create_store()?;
    main_tree_view.set_model(Some(&store));

    generate_password_menu_item.connect_activate(glib::clone!(@strong generate_password_dialog => move |_| {
        generate_password_dialog.show_all();
    }));

    generate_password_dialog_refresh_button.connect_clicked(glib::clone!(@strong builder => move |_| {
        generate_password_dialog_refresh_action(&builder);
    }));

    generate_password_dialog_cancel_button.connect_clicked(glib::clone!(@weak generate_password_dialog => move |_| {
        generate_password_dialog.hide();
    }));

    remove_menu_item.connect_activate(glib::clone!(@strong store, @weak main_tree_view, @strong main_text_view => move |_menu_item| {
        remove_menu_item_action(&store, &main_tree_view, &main_text_view);
    }));

    let popup_menu: gtk::Menu = gtk::MenuBuilder::new().child(&remove_menu_item).build();
    main_tree_view.connect_button_press_event(move |_tree_view, event| {
        if event.get_event_type() == gdk::EventType::ButtonPress && event.get_button() == 3 {
            debug!("event: {:?}", event);
            popup_menu.popup_easy(event.get_button(), event.get_time());
            popup_menu.show_all();
        }
        gtk::Inhibit(false)
    });

    new_menu_item.connect_activate(glib::clone!(@strong mc, @strong store, @weak main_tree_view  => move |a| {
        debug!("a: {}", a);
        new_menu_item_action(&mc, &store, &main_tree_view);
    }));

    let renderer = gtk::CellRendererTextBuilder::new().editable(true).build();
    renderer.connect_edited(glib::clone!(@strong main_tree_view, @strong store => move |_renderer, _path, new_title| {
        tree_view_cell_renderer_edited(new_title, &main_tree_view, &store);
    }));

    let column = gtk::TreeViewColumnBuilder::new().title("Title").sort_column_id(0i32).build();
    column.pack_start(&renderer, true);
    column.add_attribute(&renderer, "text", 0i32);

    main_tree_view.append_column(&column);

    let tree_view_selection = main_tree_view.get_selection();
    tree_view_selection.connect_changed(glib::clone!(@weak main_text_view, @strong mc => move |tree_selection| {
        tree_view_selection_changed(tree_selection, &main_text_view, &mc);
    }));

    main_text_view.connect_key_press_event(
        glib::clone!(@strong mc, @weak main_tree_view => @default-return Inhibit(false), move |text_view, _| {
            text_view_key_press_event_action(&main_tree_view, text_view, &mc);
            Inhibit(false)
        }),
    );

    import_menu_item.connect_activate(glib::clone!(@weak main_window, @strong mc, @weak store => move |_| {
        import_menu_item_action(&main_window, &mc, &store, &main_tree_view);
    }));

    export_menu_item.connect_activate(glib::clone!(@weak main_window, @strong mc => move |_| {
        export_menu_item_action(&mc);
    }));

    about_menu_item.connect_activate(glib::clone!(@weak main_window => move |_| {
        about_dialog.show();
        about_dialog.run();
        about_dialog.hide();
    }));

    quit_menu_item.connect_activate(glib::clone!(@weak main_window => move |_| {
        main_window.close();
    }));

    main_window.set_application(Some(application));

    main_window.connect_delete_event(glib::clone!(@weak main_window => @default-return Inhibit(false), move |_, _| {
        main_window.close();
        Inhibit(false)
    }));

    main_window.show_all();
    Ok(())
}

fn create_store() -> Result<gtk::ListStore, Box<dyn Error>> {
    let store = gtk::ListStore::new(&[glib::Type::String]);
    let items = item_actions::find_all(None).expect("failed to get Items");
    for item in items.iter() {
        debug!("item: {:?}", item);
        let value = glib::value::Value::from(&item.title);
        store.set_value(&store.append(), 0u32, &value);
    }
    Ok(store)
}

fn generate_password_dialog_refresh_action(builder: &gtk::Builder) {
    let generate_password_dialog_include_numbers_checkbox: gtk::CheckButton = builder.get_object("generate_password_dialog_include_numbers_checkbox").unwrap();
    let generate_password_dialog_include_uppercase_checkbox: gtk::CheckButton =
        builder.get_object("generate_password_dialog_include_uppercase_checkbox").unwrap();
    let generate_password_dialog_include_symbols_checkbox: gtk::CheckButton = builder.get_object("generate_password_dialog_include_symbols_checkbox").unwrap();
    let generate_password_dialog_length_combobox: gtk::ComboBox = builder.get_object("generate_password_dialog_length_combobox").unwrap();
    let generate_password_dialog_count_combobox: gtk::ComboBox = builder.get_object("generate_password_dialog_count_combobox").unwrap();
    let generate_password_dialog_textview: gtk::TextView = builder.get_object("generate_password_dialog_textview").unwrap();

    let generator = passwords::PasswordGenerator::new()
        .spaces(false)
        .exclude_similar_characters(true)
        .strict(true)
        .lowercase_letters(true)
        .numbers(generate_password_dialog_include_numbers_checkbox.get_active())
        .symbols(generate_password_dialog_include_symbols_checkbox.get_active())
        .uppercase_letters(generate_password_dialog_include_uppercase_checkbox.get_active())
        .length(generate_password_dialog_length_combobox.get_active_id().unwrap().parse::<usize>().unwrap());
    debug!("generator: {:?}", generator);
    let passwords = generator
        .generate(generate_password_dialog_count_combobox.get_active_id().unwrap().parse::<usize>().unwrap())
        .expect("Couldn't generate passwords");
    let generate_password_dialog_textview_buffer = generate_password_dialog_textview.get_buffer().expect("Couldn't get buffer");
    generate_password_dialog_textview_buffer.set_text(&passwords.join("\n"));
}

fn new_menu_item_action(mc: &magic_crypt::MagicCrypt256, store: &gtk::ListStore, tree_view: &gtk::TreeView) {
    let mut new_item = models::NewItem::new("New".into());
    let contents: String = "Enter text here".into();
    new_item.contents = Some(mc.encrypt_str_to_base64(contents));
    match item_actions::insert(&new_item) {
        Ok(_) => {
            let value = glib::value::Value::from(&new_item.title);
            let iter = store.append();
            store.set_value(&iter, 0u32, &value);
            let path = store.get_path(&iter).expect("Couldn't get path");
            tree_view.get_selection().select_path(&path);
        }
        Err(e) => warn!("{}", e),
    }
}

fn import_menu_item_action(main_window: &gtk::Window, mc: &magic_crypt::MagicCrypt256, store: &gtk::ListStore, tree_view: &gtk::TreeView) {
    let file_chooser_dialog = gtk::FileChooserDialogBuilder::new()
        .title("Choose a file to import")
        .show_hidden(true)
        .select_multiple(true)
        .transient_for(main_window)
        .action(gtk::FileChooserAction::Open)
        .build();

    file_chooser_dialog.add_button("Open", gtk::ResponseType::Ok);
    file_chooser_dialog.add_button("Cancel", gtk::ResponseType::Cancel);

    if file_chooser_dialog.run() == gtk::ResponseType::Ok {
        let files = file_chooser_dialog.get_filenames();
        files.iter().for_each(|z| info!("file: {}", z.to_string_lossy()));
        for path in files.iter() {
            let item_title: String = path.file_name().unwrap().to_os_string().into_string().unwrap();
            let mut new_item = models::NewItem::new(item_title);
            let contents = std::fs::read_to_string(path.as_path()).unwrap();
            new_item.contents = Some(mc.encrypt_str_to_base64(contents));
            match item_actions::insert(&new_item) {
                Ok(_) => {
                    let value = glib::value::Value::from(&new_item.title);
                    let iter = store.append();
                    store.set_value(&iter, 0u32, &value);
                    let path = store.get_path(&iter).expect("Couldn't get path");
                    tree_view.get_selection().select_path(&path);
                }
                Err(e) => warn!("{}", e),
            }
        }
    }

    file_chooser_dialog.close();
}

fn export_menu_item_action(mc: &magic_crypt::MagicCrypt256) {
    let project_dir = dirs::home_dir().unwrap().join(".senoru");
    if !project_dir.as_path().exists() {
        std::fs::create_dir_all(&project_dir).ok();
    }
    let export_dir = project_dir.join("export");
    if !export_dir.as_path().exists() {
        std::fs::create_dir_all(&export_dir).ok();
    }

    let items = item_actions::find_all(None).expect("failed to get Items");
    for item in items.iter().cloned() {
        let output_file = export_dir.join(&item.title);
        let mut bw = io::BufWriter::new(fs::File::create(output_file.as_path()).unwrap());
        let contents = item.decrypt_contents(mc).expect("failed to decrypt item");
        bw.write_all(contents.as_bytes()).expect("Unable to write data");
    }

    let info_dialog = gtk::MessageDialogBuilder::new()
        .title("Export")
        .buttons(gtk::ButtonsType::Ok)
        .message_type(gtk::MessageType::Info)
        .modal(true)
        .text(format!("Items were written to: {}", export_dir.to_string_lossy()).as_str())
        .build();
    info_dialog.run();
    info_dialog.close();
}

fn remove_menu_item_action(store: &gtk::ListStore, tree_view: &gtk::TreeView, text_view: &gtk::TextView) {
    let selection = tree_view.get_selection();
    let (model, iter) = selection.get_selected().expect("Couldn't get selected");
    let selected_title = model.get_value(&iter, 0).get::<String>().expect("failed to get selected title");

    match selected_title {
        Some(title) => {
            let item = item_actions::find_by_title(&title).expect("failed to find Item by title");
            match item {
                Some(i) => {
                    item_actions::delete(&i.id).expect("failed to delete item");
                    store.remove(&iter);
                    match store.get_iter_first() {
                        Some(_) => {}
                        None => {
                            let text_view_buffer = text_view.get_buffer().expect("Couldn't get buffer");
                            text_view_buffer.set_text(&"");
                        }
                    }
                }
                None => {}
            }
        }
        None => {}
    }
}

fn tree_view_selection_changed(tree_selection: &gtk::TreeSelection, text_view: &gtk::TextView, mc: &magic_crypt::MagicCrypt256) {
    match tree_selection.get_selected() {
        Some((model, iter)) => {
            let selected_title = model.get_value(&iter, 0).get::<String>().expect("failed to get selected title");
            let text_view_buffer = text_view.get_buffer().expect("Couldn't get buffer");
            match selected_title {
                Some(title) => {
                    let item = item_actions::find_by_title(&title).expect("failed to find Item by title");
                    match item {
                        Some(i) => {
                            text_view_buffer.set_text(&i.decrypt_contents(&mc).unwrap());
                        }
                        None => text_view_buffer.set_text(&""),
                    }
                }
                _ => text_view_buffer.set_text(&""),
            }
        }
        None => {}
    }
}

fn tree_view_cell_renderer_edited(new_title: &str, tree_view: &gtk::TreeView, store: &gtk::ListStore) {
    let selection = tree_view.get_selection();
    let (model, iter) = selection.get_selected().expect("Couldn't get selected");
    let selected_title = model.get_value(&iter, 0).get::<String>().expect("failed to get selected title");

    match selected_title {
        Some(title) => {
            let item = item_actions::find_by_title(&title).expect("failed to find Item by title");
            match item {
                Some(mut i) => {
                    i.title = new_title.to_string();
                    item_actions::update(&i).expect("failed to update item");
                    let value = glib::value::Value::from(&i.title);
                    store.set_value(&iter, 0u32, &value);
                }
                None => {}
            }
        }
        None => {}
    }
}

fn text_view_key_press_event_action(tree_view: &gtk::TreeView, text_view: &gtk::TextView, mc: &magic_crypt::MagicCrypt256) {
    let selection = tree_view.get_selection();
    let (model, iter) = selection.get_selected().expect("Couldn't get selected");
    let selected_title = model.get_value(&iter, 0).get::<String>().expect("failed to get selected title");
    match selected_title {
        Some(title) => {
            let item = item_actions::find_by_title(&title).expect("failed to find Item by title");
            match item {
                Some(mut i) => {
                    let buffer = text_view.get_buffer().expect("Couldn't get buffer");
                    let contents = buffer
                        .get_text(&buffer.get_start_iter(), &buffer.get_end_iter(), false)
                        .expect("failed to get content")
                        .to_string();
                    i.contents = Some(mc.encrypt_str_to_base64(contents));
                    item_actions::update(&i).expect("failed to update item");
                }
                None => {}
            }
        }
        None => {}
    }
}
