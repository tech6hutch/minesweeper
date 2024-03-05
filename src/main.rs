mod game_window;
// mod setup_window;
mod shared;
mod text;

use game_window::GameEnd;
use shared::Config;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let app_name = args
        .first()
        .map(|s| s.as_str())
        .unwrap_or("minesweeper.exe");
    let mut help_arg = false;
    let mut rows_arg: Option<&str> = None;
    let mut cols_arg: Option<&str> = None;
    let mut mines_arg: Option<&str> = None;
    for arg in args.iter().skip(1) {
        if matches!(arg.as_str(), "help" | "-h" | "-help" | "--help") {
            help_arg = true;
        } else if arg.starts_with("rows=") {
            rows_arg = Some(arg);
        } else if arg.starts_with("cols=") {
            cols_arg = Some(arg);
        } else if arg.starts_with("mines=") {
            mines_arg = Some(arg);
        } else {
            eprint!("Unknown flag '{arg}'. ");
            print_help(app_name);
            return;
        }
    }

    if help_arg {
        print_help(app_name);
        return;
    }

    fn parse_num_arg(arg: Option<&str>) -> Option<usize> {
        let arg = arg?;
        let equals_sign_idx = arg.find('=').expect("already checked to exist");
        let num = &arg[equals_sign_idx + 1..];
        let Ok(num) = num.parse() else {
            eprintln!("What kind of number is '{num}'?");
            return None;
        };
        Some(num)
    }

    let mut cfg = Config::default();
    if let Some(rows) = parse_num_arg(rows_arg) {
        cfg.cell_rows = rows;
    }
    if let Some(cols) = parse_num_arg(cols_arg) {
        cfg.cell_cols = cols;
    }
    if let Some(mines) = parse_num_arg(mines_arg) {
        cfg.mine_count = mines;
    }

    match game_window::run(&mut cfg) {
        GameEnd::Quit => {}
    }
}

fn print_help(app_name: &str) {
    let default_cfg = Config::default();
    eprintln!(
        "USAGE: {app_name} [rows={}] [cols={}] [mines={}]",
        default_cfg.cell_rows, default_cfg.cell_cols, default_cfg.mine_count
    );
}
