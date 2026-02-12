#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use common::app_main_vk::MainAppVk;

fn main() {
    let _ = common::graceful_run(MainAppVk::run);
}
