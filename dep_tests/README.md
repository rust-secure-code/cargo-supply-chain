# deps_tests

The files in this directory are used by tests in `../src/common.rs`.

Each of the `.metadata.json` files was generated with a command of the following form:

```sh
cargo metadata |
sed "s,${PWD},\$CARGO_MANIFEST_DIR,g" |
sed "s,${HOME},\$HOME,g" |
jq --sort-keys > ${CARGO_SUPPLY_CHAIN_DIR}/dep_tests/${PACKAGE}_${VERSION}.metadata.json
```

The other files were then generated with the following command:

```
BLESS=1 cargo test 'tests::deps'
```

Optionally, all of the `.json` files can be normalized with the following command:

```sh
for X in *.json; do
    Y="$(mktemp)"
    jq --sort-keys < "$X" > "$Y"
    mv -f "$Y" "$X"
done
```

"Optionally" because the tests should not require the `.json` files to be normalized.
