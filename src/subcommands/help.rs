//! Displays help infomation to the user when requested

use crate::CLI_HELP;
use std::process;

/// Provides help infomation which proceeds to exit
pub fn help(command: Option<&str>) {
    match command {
        None => println!("{}", CLI_HELP),
        Some("crates") => todo!(),
        Some("publishers") => todo!(),
        Some("update") => todo!(),
        Some(command) => {
            println!("Unknown subcommand: {}\n", command);
            println!("{}", CLI_HELP);
            process::exit(1)
        }
    }
    process::exit(0)
}
