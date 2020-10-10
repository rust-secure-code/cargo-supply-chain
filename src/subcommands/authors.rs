use std::collections::HashSet;

use crate::common::*;

pub fn authors(mut args: std::env::ArgsOs) {
    while let Some(arg) = args.next() {
        match arg.to_str() {
            None => bail_bad_arg(arg),
            Some("--") => break, // we pass args after this to cargo-metadata
            _ => bail_unknown_authors_arg(arg),
        }
    }

    let dependencies = sourced_dependencies(args);

    let authors: HashSet<_> = crate::authors::authors_of(&dependencies).collect();
    let mut display_authors: Vec<_> = authors.iter().map(|a| a.to_string()).collect();
    display_authors.sort_unstable();
    for a in display_authors {
        println!("{}", a);
    }
}
