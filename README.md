# cargo-supply-chain

Gather author, contributor and publisher data on crates in your dependency graph.

Use cases include:

- Find people and groups worth supporting.
- Identify risks in your dependency graph.
- An analysis of all the contributors you implicitly trust by building their software. This might have both a sobering and humbling effect.

Sample output when run on itself: [`publishers`](https://gist.github.com/Shnatsel/3b7f7d331d944bb75b2f363d4b5fb43d), [`crates`](https://gist.github.com/Shnatsel/dc0ec81f6ad392b8967e8d3f2b1f5f80), [`json`](https://gist.github.com/Shnatsel/511ad1f87528c450157ef9ad09984745).

## Usage

To install this tool, please run the following command:

```shell
cargo install cargo-supply-chain
```

Then run it with:

```shell
cargo supply-chain publishers
```

By default the supply chain is listed for **all targets** and **default features only**.

You can alter this behavior by passing `--target=…` to list dependencies for a specific target.
You can use `--all-features`, `--no-default-features`, and `--features=…` to control feature selection.

Here's a list of subcommands:

```none
Gather author, contributor and publisher data on crates in your dependency graph

Usage: COMMAND [ARG]…

Available options:
    -h, --help      Prints help information
    -v, --version   Prints version information

Available commands:
    publishers  List all crates.io publishers in the depedency graph
    crates      List all crates in dependency graph and crates.io publishers for each
    json        Like 'crates', but in JSON and with more fields for each publisher
    update      Download the latest daily dump from crates.io to speed up other commands

Most commands also accept flags controlling the features, targets, etc.
See 'cargo supply-chain <command> --help' for more information on a specific command.
```

## Colorful line parser output

You can install `cargo-supply-chain` with one of two features to get prettier command line
```console
cargo install cargo-supply-chain -F bright-color
cargo install cargo-supply-chain -F dull-color
```

## License

Triple licensed under any of Apache-2.0, MIT, or zlib terms.
