//! Displays help infomation to the user when requested

use crate::CLI_HELP;
use std::process;

/// Provides help infomation which proceeds to exit
pub fn help(command: Option<&str>) {
    match command {
        Some("crates") => todo!(),
        Some("publishers") => todo!(),
        Some("update") => todo!(),
        Some(_) => println!("{}", CLI_HELP),
        None => println!("{}", CLI_HELP),
    }
    process::exit(0)
}
