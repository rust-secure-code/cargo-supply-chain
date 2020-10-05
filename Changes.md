## v0.0.1 (2020-10-02)

Initial release, supports one command:
- `authors`: Crawl through Cargo.toml of all crates and list their authors.
  Authors might be listed multiple times. For each author, differentiate if
  they are known by being mentioned in a crate from the local workspace or not.
  Support for crawling `crates.io` sourced packages is planned.
- `publishers`: Doesn't do anything right now.
