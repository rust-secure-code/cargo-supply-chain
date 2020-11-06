# cargo-supply-chain

Gather author, contributor, publisher data on crates in your dependency graph.

Use cases include:

* Find people and groups worth supporting.
* An analysis of all the contributors you implicitly trust by building their software. This
  might have both a sobering and humbling effect.
* Identify risks in your dependency graph.

## Usage

Run `cargo install cargo-supply-chain` to install the tool. Navigate to your project and run `cargo supply-chain` followed by a subcommand, e.g. `cargo supply-chain publishers`

### Subcommands

 * `authors` - lists all the authors for all dependencies, as specified in `Cargo.toml` files. Works offline.
 * `publishers` - lists all the people and teams that can publish updates to your dependencies on crates.io.
 * `crates` - lists all the crates you depend on, with the list of publishers for each crate.
 * `update` - downloads a daily database dump of crates.io (roughly 256Mb) to speed up `publishers` and `crates` subcommands. Data downloaded this way may be out of date by up to 48 hours. You can set the maximum allowed age using the `--cache-max-age` flag; if it's exceeded, live data will be fetched instead.

### Filtering

Any arguments specified after `--` will be passed to `cargo metadata`, for example:

  `cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu`

will only include dependencies that are used when compiling for `x86_64-unknown-linux-gnu` and ignore crates that are not used on this platform (e.g. `winapi`, `web-sys`).

See `cargo metadata --help` for more info.

## License

Triple licensed under any of Apache-2.0, MIT, or zlib terms.
