# A Rust JSON structural diff CLI

[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Actions Status](https://github.com/Luni-4/json-structural-diff/workflows/json-structural-diff/badge.svg)](https://github.com/Luni-4/json-structural-diff/actions)
[![Coverage Status](https://coveralls.io/repos/github/Luni-4/json-structural-diff/badge.svg?branch=master)](https://coveralls.io/github/Luni-4/json-structural-diff?branch=master)

A pure-Rust JSON structural diff CLI based on the JSON structural diff library.

## Building CLI

```bash
cargo build --workspace
```

If you want to build the cli in release mode, add the `--release` option
to the command above.

## Installing CLI

Run `cargo install json-diff-cli` or download the binaries contained in the
[release](https://github.com/Luni-4/json-structural-diff/releases/) page.

## License

Released under the [MIT License](../LICENSE).
