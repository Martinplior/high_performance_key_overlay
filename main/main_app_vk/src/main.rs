#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use common::main_app_vk::MainAppVk;

fn main() {
    let _ = common::graceful_run(MainAppVk::run);
}
