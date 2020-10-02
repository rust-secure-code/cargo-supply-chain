//! Gather author, contributor, owner data on crates in your dependency graph.
//!
//! There are some use cases:
//!
//! * Find people and groups worth supporting.
//! * An analysis of all the contributors you implicitly trust by building their software. This
//!   might have both a sobering and humbling effect.
//! * Identify risks in your dependency graph.

fn main() {
    let mut args = std::env::args_os().skip(1);

    while let Some(arg) = args.next() {
        match arg.to_str() {
            None => bail_bad_arg(arg),
            Some("authors") => todo!(),
            Some("owners") => todo!(),
            Some(arg) if arg.starts_with("--") => bail_unknown_option(arg),
            Some(arg) if arg.starts_with("-") => bail_unknown_short_option(arg),
            Some(arg) => bail_unknown_command(arg),
        }
    }

    // No tool selected.
    bail_no_tool();
}

fn bail_unknown_option(arg: &str) -> ! {
    eprintln!("Unknown option: {}", std::path::Path::new(&arg).display());
    std::process::exit(1);
}

fn bail_unknown_short_option(arg: &str) -> ! {
    eprintln!("Unknown flag: {}", arg);
    std::process::exit(1);
}

fn bail_unknown_command(arg: &str) -> ! {
    eprintln!("Unknown command: {}", arg);
    std::process::exit(1);
}

fn bail_bad_arg(arg: std::ffi::OsString) -> ! {
    eprintln!("Bad argument: {}", std::path::Path::new(&arg).display());
    std::process::exit(1);
}

fn bail_no_tool() -> ! {
    eprintln!("No tool selected.");
    eprint_help();
    std::process::exit(1);
}

fn eprint_help() {
    eprintln!("Usage: cargo supply-chain COMMAND [OPTIONS...]\n

  Commands:
    authors\t\tList all authors in the dependency graph\n
    owners\t\tList all owners in the dependency graph\n
");
}
