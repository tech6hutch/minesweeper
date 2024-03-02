mod game_window;
mod setup_window;
mod shared;
mod text;

use game_window::GameEnd;
use shared::Config;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mut rows_arg: Option<&str> = None;
    let mut cols_arg: Option<&str> = None;
    let mut mines_arg: Option<&str> = None;
    for arg in args.iter().skip(1) {
        if arg.starts_with("rows=") {
            rows_arg = Some(arg);
        } else if arg.starts_with("cols=") {
            cols_arg = Some(arg);
        } else if arg.starts_with("mines=") {
            mines_arg = Some(arg);
        } else {
            eprintln!("Unknown flag '{arg}'");
            return;
        }
    }
    let any_flags = args.len() > 1;

    let mut cfg: Option<Config> = if any_flags {
        let mut config = Config::default();
        if let Some(rows) = rows_arg {
            let last_non_digit = rows
                .char_indices()
                .rev()
                .find_map(|(i, c)| (!c.is_ascii_digit()).then_some(i))
                .expect("this starts with a non digit");
            let num = &rows[last_non_digit + 1..];
            let Ok(num) = num.parse() else {
                eprintln!("What kind of number is '{num}'?");
                return;
            };
            config.cell_rows = num;
        }
        if let Some(cols) = cols_arg {
            let last_non_digit = cols
                .char_indices()
                .rev()
                .find_map(|(i, c)| (!c.is_ascii_digit()).then_some(i))
                .expect("this starts with a non digit");
            let num = &cols[last_non_digit + 1..];
            let Ok(num) = num.parse() else {
                eprintln!("What kind of number is '{num}'?");
                return;
            };
            config.cell_cols = num;
        }
        if let Some(mines) = mines_arg {
            let last_non_digit = mines
                .char_indices()
                .rev()
                .find_map(|(i, c)| (!c.is_ascii_digit()).then_some(i))
                .expect("this starts with a non digit");
            let num = &mines[last_non_digit + 1..];
            let Ok(num) = num.parse() else {
                eprintln!("What kind of number is '{num}'?");
                return;
            };
            config.mine_count = num;
        }
        Some(config)
    } else {
        None
    };
    loop {
        cfg = setup_window::run(cfg);
        let Some(cfg) = cfg.as_mut() else {
            return;
        };
        match game_window::run(cfg) {
            GameEnd::Restart => {}
            GameEnd::Quit => return,
        }
    }
}
