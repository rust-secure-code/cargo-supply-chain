# cargo-supply-chain

Gather author, contributor and publisher data on crates in your dependency graph.

Use cases include:

- Find people and groups worth supporting.
- Identify risks in your dependency graph.
- An analysis of all the contributors you implicitly trust by building their software. This might have both a sobering and humbling effect.

## Usage

Run `cargo install cargo-supply-chain` to install this tool.

Once installed, simply navigate to your project and run `cargo supply-chain` followed by a subcommand, e.g. `cargo supply-chain publishers`.

### Subcommands

- `publishers` - Lists all the people and teams that can publish updates to your dependencies on crates.io.
- `crates` - Lists all the crates you depend on, with the list of publishers for each crate.
- `update` - Downloads a daily database dump of crates.io (roughly 256Mb) to speed up `publishers` and `crates` subcommands. Data downloaded this way may be out of date by up to 48 hours. You can set the maximum allowed age using the `--cache-max-age` flag; if it's exceeded, live data will be fetched instead.
- `help` - Displays detailed help for a specific command.

### Filtering

Any arguments specified after `--` will be passed to `cargo metadata`, for example:

```none
cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu
```

This will only include dependencies that are used when compiling for `x86_64-unknown-linux-gnu` and ignore crates that are not used on this platform (e.g. `winapi`, `web-sys`).

See `cargo metadata --help` for a list of flags it supports.

## License

Triple licensed under any of Apache-2.0, MIT, or zlib terms.
