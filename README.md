<p align="center">
  <a href="https://github.com/nvzqz/aloxide">
    <img width="800" src="https://github.com/nvzqz/aloxide/raw/assets/aloxide_banner.svg?sanitize=true" alt="aloxide banner">
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

- [x] Linux

- [x] macOS

- [ ] Windows

  - [ ] [Microsoft Visual C (MSVC)][MSVC] toolchain

  - [ ] [GNU Compiler Collection (GCC)][GCC] toolchain

## Cross-Compiling

Work in progress...

## License

This project is released under either:

- [MIT License](https://github.com/nvzqz/static-assertions-rs/blob/master/LICENSE-MIT)

- [Apache License (Version 2.0)](https://github.com/nvzqz/static-assertions-rs/blob/master/LICENSE-APACHE)

at your choosing.

[CLI]:  https://en.wikipedia.org/wiki/Command-line_interface
[MSVC]: https://en.wikipedia.org/wiki/MSVC
[GCC]:  https://en.wikipedia.org/wiki/GNU_Compiler_Collection
