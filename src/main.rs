#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod game_window;
// mod setup_window;
mod shared;
mod text;

use game_window::GameEnd;
use shared::Config;

fn main() {
    const DIED_FILE_PATH: &str = "./.cfg";
    const DIED_VALUE: [u8; 4] = 0xDEAD_BEEF_u32.to_be_bytes();

    let mut cfg = Config::default();
    if let Ok(bytes) = std::fs::read(DIED_FILE_PATH) {
        if bytes == DIED_VALUE {
            cfg.already_died = true;
        }
    }

    match game_window::run(&mut cfg) {
        GameEnd::DidNotLose => {}
        GameEnd::Lost => {
            std::fs::write(DIED_FILE_PATH, DIED_VALUE).unwrap();
        }
    }
}
