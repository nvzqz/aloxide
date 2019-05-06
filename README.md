<p align="center">
  <a href="https://github.com/nvzqz/aloxide">
    <img width="800" src="https://github.com/nvzqz/aloxide/raw/assets/aloxide_banner.svg?sanitize=true" alt="aloxide banner">
  </a>
  <a href="https://travis-ci.com/nvzqz/aloxide">
    <img src="https://travis-ci.com/nvzqz/aloxide.svg?branch=master" alt="travis badge">
  </a>
  <a href="https://crates.io/crates/aloxide">
    <img src="https://img.shields.io/crates/v/aloxide.svg" alt="crates.io">
    <img src="https://img.shields.io/crates/d/aloxide.svg" alt="downloads">
  </a>
  <a href="https://docs.rs/aloxide">
    <img src="https://docs.rs/aloxide/badge.svg" alt="API docs">
  </a>
</p>

Compile Ruby as a Rust `build.rs` step... and eventually more (see
[goals](#goals)).

## Goals

The plan for `aloxide` is to:

- Compile Ruby for each [supported platform](#supported-platforms)

- Link to Ruby's library in a crate's `build.rs` file

- Make pre-compiled Rubies that are suitable for various versions of the same
  operating system

- [Cross-compile](#cross-compiling) Ruby from one platform to another

  - Compile for `{i686,x86_64}-pc-windows-gnu` from Linux or macOS

  - Compile for `{i686,x86_64}-unknown-linux-gnu` from macOS or Windows

- Create a [command-line interface (CLI)][CLI] that downloads Ruby's sources and
  compiles them, or downloads pre-compiled binaries/libraries, for each
  [supported platform](#supported-platforms)

## Supported Platforms

See [issue #1](https://github.com/nvzqz/aloxide/issues/1) for more details.

- [x] Linux

- [x] macOS

- [ ] Windows

## Cross-Compiling

Work in progress...

## License

This project is released under either:

- [MIT License](https://github.com/nvzqz/static-assertions-rs/blob/master/LICENSE-MIT)

- [Apache License (Version 2.0)](https://github.com/nvzqz/static-assertions-rs/blob/master/LICENSE-APACHE)

at your choosing.

[CLI]: https://en.wikipedia.org/wiki/Command-line_interface
