//! Displays help infomation to the user when requested

use crate::CLI_HELP;
use std::process;

/// Provides help infomation which proceeds to exit
pub fn help() {
    println!("{}", CLI_HELP);
    process::exit(0)
}
