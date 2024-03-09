#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<(), eframe::Error> {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([725.0, 1000.0])
            .with_min_inner_size([725.0, 1000.0]),
        ..Default::default()
    };

    eframe::run_native(
        "BingoSync JSON Generator",
        native_options,
        Box::new(|cc| Box::new(bingosync_gen::BingoSyncGen::new(cc))),
    )
}
