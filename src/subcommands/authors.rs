use std::collections::HashSet;

use crate::common::*;

pub fn authors(args : Vec<String>) {
    let dependencies = sourced_dependencies(args);

    let authors: HashSet<_> = crate::authors::authors_of(&dependencies).collect();
    let mut display_authors: Vec<_> = authors.iter().map(|a| a.to_string()).collect();
    display_authors.sort_unstable();
    for a in display_authors {
        println!("{}", a);
    }
}
