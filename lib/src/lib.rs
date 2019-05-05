//! <p align="center">
//!   <a href="https://github.com/nvzqz/aloxide">
//!     <img width="800" src="https://github.com/nvzqz/aloxide/raw/assets/aloxide_banner.svg?sanitize=true" alt="aloxide banner">
//!   </a>
//! </p>
//!
//!
//! Download, configure, compile, and link to Ruby.
//!
//! # Usage
//!
//! This crate is available [on crates.io][crate] and can be used by adding the
//! following to your project's [`Cargo.toml`]:
//!
//! ```toml
//! [build-dependencies]
//! aloxide = "0.0.2"
//! ```
//!
//! _or_ if you're mental and would like to be on the latest and... um greatest
//! (?), then you can depend directly on the GitHub repository:
//!
//! ```toml
//! [build-dependencies]
//! aloxide = { git = "https://github.com/nvzqz/aloxide" }
//! ```
//!
//! and finally add this to your Cargo build script (`build.rs`):
//!
//! ```
//! extern crate aloxide;
//! # fn main() {}
//! ```
//!
//! # Supported Platforms
//!
//! Currently, `aloxide` only supports Linux and macOS. See
//! [issue #1](https://github.com/nvzqz/aloxide/issues/1) for more details.
//!
//! # Examples
//!
//! Given a directory of sources, Ruby can be built as such:
//!
//! ```rust,no_run
//! use aloxide::Ruby;
//!
//! // When necessary, `rustc` targets are made build-compatible for Ruby
//! let target = std::env::var("TARGET").unwrap();
//!
//! let src_dir = "path/to/sources";
//! let out_dir = "path/to/build";
//!
//! let ruby = Ruby::builder(src_dir, out_dir, target)
//!     .configure()      // Change what happens when running `configure`
//!         .inherit_cc() // Use the `CC` environment variable
//!     .make()           // Change what happens when running `make`
//!         .force()      // Always run `make` regardless if sources built
//!     .build()          // Run all build steps
//!     .unwrap();
//!
//! let hello_world = ruby.run("puts 'Hello, World!").unwrap();
//! assert_eq!(hello_world, "Hello, World!\n");
//! ```
//!
//! Ruby can linked to the current crate very easily:
//!
//! ```rust,no_run
//! # let ruby: aloxide::Ruby = unimplemented!();
//! // Link Ruby statically
//! if let Err(error) = ruby.link(true) {
//!     // Handle `error`
//! }
//! ```
//!
//! [crate]: https://crates.io/crates/aloxide
//! [`Cargo.toml`]: https://doc.rust-lang.org/cargo/reference/manifest.html

#![deny(missing_docs)]

extern crate bzip2;
extern crate cc;
extern crate dirs;
extern crate memchr;
extern crate tar;

#[cfg(feature = "ureq")]
extern crate ureq;

use std::ffi::OsStr;
use std::fmt::Display;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::string::FromUtf8Error;

mod archive;
mod link;
pub mod build;
pub mod version;

#[cfg(feature = "download")] pub mod download;
#[cfg(feature = "download")] pub use download::RubySrcDownloader;

use build::RubyBuildError;

#[doc(inline)]
pub use self::{
    archive::Archive,
    build::RubyBuilder,
    link::*,
    version::Version,
};

/// An existing Ruby installation
///
/// Ruby's sources are located in [`src_dir`](#method.src_dir) and its build
/// output in [`out_dir`](#method.out_dir).
#[derive(Debug)]
pub struct Ruby {
    version: Version,
    src_dir: PathBuf,
    out_dir: PathBuf,
    lib_path: PathBuf,
    bin_path: PathBuf,
}

impl Ruby {
    /// Returns a new Ruby source code downloader.
    #[inline]
    #[cfg(feature = "download")]
    pub fn src_downloader<'a, P: AsRef<Path> + ?Sized>(
        version: Version,
        dst_dir: &'a P,
    ) -> RubySrcDownloader<'a> {
        RubySrcDownloader::new(version, dst_dir.as_ref())
    }

    /// Downloads and unpacks the source for `version` to `dst_dir` with the
    /// default configuration.
    #[inline]
    #[cfg(feature = "download")]
    pub fn download_src(
        version: Version,
        dst_dir: impl AsRef<Path>,
    ) -> Result<PathBuf, download::RubySrcDownloadError> {
        Self::src_downloader(version, dst_dir.as_ref()).download()
    }

    /// Returns a new Ruby builder.
    #[inline]
    pub fn builder(
        src_dir: impl Into<PathBuf>,
        out_dir: impl Into<PathBuf>,
        target: impl AsRef<str>,
    ) -> RubyBuilder {
        RubyBuilder::new(src_dir.into(), out_dir.into(), target.as_ref())
    }

    /// Builds Ruby with the default configuration.
    #[inline]
    pub fn build(
        src_dir: impl Into<PathBuf>,
        out_dir: impl Into<PathBuf>,
        target: impl AsRef<str>,
    ) -> Result<Self, RubyBuildError> {
        Self::builder(src_dir, out_dir, target).build()
    }

    /// Creates a new instance without doing anything.
    #[inline]
    pub fn new(
        version: Version,
        src_dir: impl Into<PathBuf>,
        out_dir: impl Into<PathBuf>,
    ) -> Ruby {
        let src_dir = src_dir.into();
        let out_dir = out_dir.into();
        let lib_path = out_dir.join("lib");
        let bin_path = out_dir.join("bin").join("ruby");
        Ruby { version, src_dir, out_dir, lib_path, bin_path }
    }

    /// Returns the Ruby version.
    #[inline]
    pub fn version(&self) -> Version {
        self.version
    }

    /// Returns the result of executing `ruby -v`.
    pub fn full_version(&self) -> Result<String, RubyExecError> {
        self.exec(Some("-v"))
    }

    /// The directory of Ruby's source code.
    #[inline]
    pub fn src_dir(&self) -> &Path {
        &self.src_dir
    }

    /// The directory of Ruby's installed files.
    #[inline]
    pub fn out_dir(&self) -> &Path {
        &self.out_dir
    }

    /// The path of the `ruby` executable.
    #[inline]
    pub fn bin_path(&self) -> &Path {
        &self.bin_path
    }

    /// Returns the output of `make` with `args`.
    pub fn make<I, S>(&self, args: I) -> io::Result<Output>
    where
        I: IntoIterator<Item=S>,
        S: AsRef<OsStr>,
    {
        Command::new("make")
            .args(args)
            .current_dir(self.src_dir())
            .output()
    }

    /// Returns the output of `make check`, which checks whether the compiled
    /// Ruby interpreter works well.
    pub fn check(&self) -> io::Result<Output> {
        self.make(&["check"])
    }

    /// Executes the `ruby` binary at `bin_path` with `args`.
    pub fn exec<I, S>(&self, args: I) -> Result<String, RubyExecError>
    where
        I: IntoIterator<Item=S>,
        S: AsRef<OsStr>,
    {
        RubyExecError::process(Command::new(&self.bin_path).args(args))
    }

    /// Runs `script` through the `ruby` interpreter at `bin_path`.
    pub fn run(&self, script: impl AsRef<OsStr>) -> Result<String, RubyExecError> {
        self.exec(&["-e".as_ref(), script.as_ref()])
    }

    /// Runs multiple scripts through the `ruby` interpreter at `bin_path`
    /// separate from one another and returns their concatenated outputs.
    ///
    /// This is the same as doing:
    ///
    /// ```sh
    /// ruby -e $script1 -e $script2 -e $script3 ...
    /// ```
    pub fn run_multiple<I, S>(&self, scripts: I) -> Result<String, RubyExecError>
    where
        I: IntoIterator<Item=S>,
        S: AsRef<OsStr>,
    {
        let flags = std::iter::repeat("-e");
        let pairs = flags.zip(scripts);

        let mut command = Command::new(&self.bin_path);
        for (flag, script) in pairs {
            command.arg(flag);
            command.arg(script);
        }

        RubyExecError::process(&mut command)
    }

    fn _get_config(&self, key: &dyn Display) -> Result<String, RubyExecError> {
        self.run(&format!("print RbConfig::CONFIG['{}']", key))
    }

    /// Returns the configuration value for `key`.
    #[inline]
    pub fn get_config(&self, key: impl Display) -> Result<String, RubyExecError> {
        self._get_config(&key)
    }

    /// Returns the `include` directory.
    #[inline]
    pub fn include_dir(&self) -> Result<String, RubyExecError> {
        self.get_config("includedir")
    }

    /// Returns the directory containing the Ruby library's main header files.
    #[inline]
    pub fn header_dir(&self) -> Result<String, RubyExecError> {
        self.get_config("rubyhdrdir")
    }

    /// Returns the directory containing the Ruby library's
    /// architecture-specific header files.
    #[inline]
    pub fn arch_header_dir(&self) -> Result<String, RubyExecError> {
        self.get_config("rubyarchhdrdir")
    }

    /// Returns the value of `RbConfig::CONFIG['LIBRUBYARG']`.
    #[inline]
    pub fn lib_args(&self) -> Result<String, RubyExecError> {
        self.get_config("LIBRUBYARG")
    }

    /// Tells `cargo` to link to Ruby and its libraries.
    pub fn link(&self, static_lib: bool) -> Result<(), RubyLinkError> {
        link::link(self, static_lib)
    }
}

/// The error returned when running `ruby` fails.
#[derive(Debug)]
pub enum RubyExecError {
    /// An IO error occurred when executing `ruby`.
    Io(io::Error),
    /// The `ruby` executable exited with a failure.
    RunFail(Output),
    /// The output of the config key is not encoded as UTF-8.
    Utf8Error(FromUtf8Error),
}

impl From<io::Error> for RubyExecError {
    #[inline]
    fn from(error: io::Error) -> Self {
        RubyExecError::Io(error)
    }
}

impl From<FromUtf8Error> for RubyExecError {
    #[inline]
    fn from(error: FromUtf8Error) -> Self {
        RubyExecError::Utf8Error(error)
    }
}

impl RubyExecError {
    fn process(command: &mut Command) -> Result<String, Self> {
        let output = command.output()?;
        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?)
        } else {
            Err(RubyExecError::RunFail(output))
        }
    }
}

