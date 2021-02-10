//! Displays help infomation to the user when requested

use crate::CLI_HELP;
use std::process;

/// Provides help infomation which proceeds to exit
pub fn help(command: Option<String>) {
    match command {
        None => println!("{}", CLI_HELP),
        Some(Command) => todo!(),
    }
    process::exit(0)
}
