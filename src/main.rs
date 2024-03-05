#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use faena_etiquetas::{constants, App};

#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "runfo_canvas",
                web_options,
                Box::new(|cc| Box::new(App::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    setup_logger();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_min_inner_size([1632.0, 664.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .unwrap(),
            ),
        ..Default::default()
    };

    let app_rc = eframe::run_native(
        "etiquetas faena",
        native_options,
        Box::new(|cc| Box::new(async_std::task::block_on(App::new(cc)))),
    );

    clean_logs();
    app_rc
}

#[inline(always)]
fn setup_logger() {
    std::env::set_var("RUST_LOG", "error");

    #[cfg(windows)]
    let log_path = std::env::var("APPDATA").expect("No APPDATA directory") + constants::LOG_FOLDER;
    if let Err(err) = std::fs::create_dir_all(&log_path) {
        eprintln!("{err}")
    }

    let file_appender = tracing_appender::rolling::daily(log_path, constants::LOG_FILENAME);
    let offset = time::UtcOffset::current_local_offset().unwrap();
    let timer = tracing_subscriber::fmt::time::OffsetTime::new(
        offset,
        time::macros::format_description!("[day]/[month]/[year] - [hour]:[minute]:[second] ||"),
    );

    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(false)
        .with_timer(timer)
        .with_writer(file_appender)
        .init();
}

#[cfg(windows)]
#[inline(always)]
/// Limpia los logs vacios
fn clean_logs() {
    use std::os::windows::fs::MetadataExt;
    let log_path = std::env::var("APPDATA").expect("No APPDATA directory") + constants::LOG_FOLDER;

    std::fs::read_dir(log_path)
        .unwrap()
        .filter_map(|res| res.ok())
        .filter(|dir_entry| {
            dir_entry
                .path()
                .to_str()
                .is_some_and(|file| file.contains(constants::LOG_FILENAME))
        })
        .filter(|dir_entry| dir_entry.metadata().unwrap().file_size() == 0)
        .for_each(|log| {
            let _ = std::fs::remove_file(log.path());
        });
}
