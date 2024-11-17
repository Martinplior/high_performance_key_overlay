#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use common::main_app::MainApp;

fn main() {
    let _ = common::graceful_run(|| MainApp::new().run());
}
