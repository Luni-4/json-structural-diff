# A Rust JSON structural diff

[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Actions Status](https://github.com/Luni-4/json-structural-diff/workflows/json-structural-diff/badge.svg)](https://github.com/Luni-4/json-structural-diff/actions)
[![Coverage Status](https://coveralls.io/repos/github/Luni-4/json-structural-diff/badge.svg?branch=master)](https://coveralls.io/github/Luni-4/json-structural-diff?branch=master)

A pure-Rust JSON structural diff based on [this](https://github.com/andreyvit/json-diff)
implementation.

This project has been developed with the aim of testing parallelism.

## Building library

```bash
cargo build
```

To build with the `colorize` feature:

```bash
cargo build --all-features
```

If you want to build the lib in release mode, add the `--release` optio
to the commands above.

## License

Released under the [MIT License](LICENSE).
