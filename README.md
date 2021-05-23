# cargo-supply-chain

Gather author, contributor and publisher data on crates in your dependency graph.

Use cases include:

- Find people and groups worth supporting.
- Identify risks in your dependency graph.
- An analysis of all the contributors you implicitly trust by building their software. This might have both a sobering and humbling effect.

## Usage

To install this tool, please run the following command:

```shell
cargo install cargo-supply-chain
```

Once installed, simply navigate to your project and run `cargo supply-chain` to start. Here's a list of possible subcommands and arguments which you may use:

```none
Commands:
  publishers   List all crates.io publishers in the depedency graph
  crates       List all crates in dependency graph and crates.io publishers for each
  json         Like 'crates', but in JSON and with more fields for each publisher
  update       Download the latest daily dump from crates.io to speed up other commands

See 'cargo supply-chain help <command>' for more information on a specific command.

Arguments:
  --cache-max-age  The cache will be considered valid while younger than specified.
                   The format is a human readable duration such as `1w` or `1d 6h`.
                   If not specified, the cache is considered valid for 48 hours.
  -d, --diffable   Make output more friendly towards tools such as `diff`

Any arguments after the `--` will be passed to `cargo metadata`, for example:
  cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu
See `cargo metadata --help` for a list of flags it supports.
```

Sample output when run on itself: [`publishers`](https://gist.github.com/Shnatsel/3b7f7d331d944bb75b2f363d4b5fb43d), [`crates`](https://gist.github.com/Shnatsel/dc0ec81f6ad392b8967e8d3f2b1f5f80), [`json`](https://gist.github.com/Shnatsel/511ad1f87528c450157ef9ad09984745).

## License

Triple licensed under any of Apache-2.0, MIT, or zlib terms.
