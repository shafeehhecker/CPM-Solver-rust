// main.rs — Entry point
// Enterprise CPM Scheduler — Rust rebuild
//
// Requires: Rust 1.78+, egui 0.27
// Build:    cargo build --release
// Test:     cargo test
// Run:      cargo run --release

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod activity;
mod scheduler;
mod project;
mod app;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1280.0, 800.0)),
        min_window_size:     Some(egui::vec2(900.0,  600.0)),
        ..Default::default()
    };

    eframe::run_native(
        "CPM Scheduler",
        native_options,
        Box::new(|cc| Box::new(app::CpmApp::new(cc))),
    )
}
