# A Rust JSON structural diff

[![Actions Status][actions badge]][actions]
[![CodeCov][codecov badge]][codecov]
[![LICENSE][license badge]][license]

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

If you want to build the lib in release mode, add the `--release` option
to the commands above.

## License

Released under the [MIT License](LICENSE).

<!-- Links -->
[actions]: https://github.com/Luni-4/json-structural-diff/actions
[codecov]: https://codecov.io/gh/Luni-4/json-structural-diff
[license]: LICENSE

<!-- Badges -->
[actions badge]: https://github.com/Luni-4/json-structural-diff/workflows/json-structural-diff/badge.svg
[codecov badge]: https://codecov.io/gh/Luni-4/json-structural-diff/branch/master/graph/badge.svg
[license badge]: https://img.shields.io/badge/license-MIT-blue.svg
