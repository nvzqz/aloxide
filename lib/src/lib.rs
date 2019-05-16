//! <p align="center">
//!   <a href="https://github.com/nvzqz/aloxide">
//!     <img width="800" src="https://github.com/nvzqz/aloxide/raw/assets/aloxide_banner.svg?sanitize=true" alt="aloxide banner">
//!   </a>
//!   </a>
//!   <a href="https://travis-ci.com/nvzqz/aloxide">
//!     <img src="https://travis-ci.com/nvzqz/aloxide.svg?branch=master" alt="travis badge">
//!   </a>
//!   <a href="https://crates.io/crates/aloxide">
//!     <img src="https://img.shields.io/crates/v/aloxide.svg" alt="crates.io">
//!     <img src="https://img.shields.io/crates/d/aloxide.svg" alt="downloads">
//!   </a>
//!   <a href="https://docs.rs/aloxide">
//!     <img src="https://docs.rs/aloxide/badge.svg" alt="API docs">
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
//! aloxide = "0.0.6"
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
//! let ruby = Ruby::src(src_dir)
//!     .builder(out_dir, target)
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

#[cfg(target_os = "windows")]
extern crate cc;

#[cfg(feature = "archive")]
extern crate bzip2;
#[cfg(feature = "archive")]
extern crate tar;

#[cfg(feature = "download")]
extern crate dirs;
#[cfg(feature = "download")]
extern crate ureq;

#[cfg(feature = "memchr")]
extern crate memchr;

use std::ffi::OsStr;
use std::fmt::{self, Display};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::string::FromUtf8Error;

#[cfg(feature = "archive")]
mod archive;
#[cfg(feature = "archive")]
pub use archive::Archive;

mod link;
mod util;
pub mod src;
pub mod version;

use version::RubyVersionError;

#[doc(inline)]
pub use self::{
    link::*,
    src::RubySrc,
    version::Version,
};

/// An existing Ruby installation
///
/// Ruby's sources are located in [`src_dir`](#method.src_dir) and its build
/// output in [`out_dir`](#method.out_dir).
#[derive(Debug)]
pub struct Ruby {
    version: Version,
    out_dir: PathBuf,
    lib_dir: PathBuf,
    bin_path: PathBuf,
}

impl Ruby {
    #[inline]
    fn bin_name() -> &'static str {
        let bin = "ruby.exe";
        if cfg!(target_os = "windows") {
            bin
        } else {
            // Use the same static string
            unsafe { bin.get_unchecked(..4) }
        }
    }

    /// Returns a `RubySrc` instance that can be used to download and build Ruby
    /// sources at `path`.
    #[inline]
    pub fn src<P: AsRef<Path> + ?Sized>(path: &P) -> &RubySrc {
        RubySrc::new(path)
    }

    /// Creates a new instance without doing anything.
    #[inline]
    pub fn new(
        version: Version,
        out_dir: impl Into<PathBuf>,
    ) -> Ruby {
        let out_dir = out_dir.into();
        let lib_dir = out_dir.join("lib");
        let bin_path = out_dir.join("bin").join(Self::bin_name());
        Ruby { version, out_dir, lib_dir, bin_path }
    }

    /// Returns the current Ruby found in `PATH`.
    #[inline]
    pub fn current() -> Result<Ruby, RubyVersionError> {
        Self::from_bin(Self::bin_name())
    }

    /// Creates a new instance from the specified `ruby` binary.
    #[inline]
    pub fn from_bin(ruby: impl AsRef<OsStr>) -> Result<Ruby, RubyVersionError> {
        Self::from_cmd(&mut Command::new(ruby))
    }

    /// Creates a new instance from executing `ruby`.
    #[inline]
    pub fn from_cmd(ruby: &mut Command) -> Result<Ruby, RubyVersionError> {
        Ruby::from_path(RubyExecError::process(
            ruby.args(&["-e", "print RbConfig::CONFIG['prefix']"])
        )?)
    }

    /// Creates a new instance, finding out the version by running the `ruby`
    /// executable in `out_dir`.
    #[inline]
    pub fn from_path(out_dir: impl Into<PathBuf>) -> Result<Ruby, RubyVersionError> {
        let mut ruby = Ruby::new(Version::new(0, 0, 0), out_dir);
        ruby.version = Version::from_bin(&ruby.bin_path)?;
        Ok(ruby)
    }

    /// Creates a new instance from the `ruby` binary installed via
    /// [`rvm`](https://github.com/rvm/rvm).
    #[inline]
    pub fn from_rvm(version: &Version) -> Result<Ruby, RubyVersionError> {
        Ruby::from_cmd(Command::new("rvm")
            .arg(version.to_string())
            .arg("do")
            .arg("ruby"))
    }

    /// Creates a new instance from the `ruby` binary installed via
    /// [`rbenv`](https://github.com/rbenv/rbenv).
    #[inline]
    pub fn from_rbenv(version: &Version) -> Result<Ruby, RubyVersionError> {
        Ruby::from_cmd(Command::new("rbenv")
            .env("RBENV_VERSION", version.to_string())
            .arg("exec")
            .arg("ruby"))
    }

    /// Returns the Ruby version.
    #[inline]
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Returns the result of executing `ruby -v`.
    pub fn full_version(&self) -> Result<String, RubyExecError> {
        self.exec(Some("-v"))
    }

    /// The directory of Ruby's installed files.
    #[inline]
    pub fn out_dir(&self) -> &Path {
        &self.out_dir
    }

    /// The directory where Ruby's library lives.
    #[inline]
    pub fn lib_dir(&self) -> &Path {
        &self.lib_dir
    }

    /// The path of the `ruby` executable.
    #[inline]
    pub fn bin_path(&self) -> &Path {
        &self.bin_path
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
        let mut command = Command::new(&self.bin_path);
        for script in scripts {
            command.arg("-e");
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

    /// Returns the name of the Ruby library.
    #[inline]
    pub fn lib_name(&self, static_lib: bool) -> Result<String, RubyExecError> {
        let mut name = self.get_config("RUBY_SO_NAME")?;
        if static_lib {
            name.push_str("-static");
        }
        Ok(name)
    }

    /// Returns the value of `RbConfig::CONFIG['LIBRUBYARG']`.
    #[inline]
    pub fn lib_args(&self) -> Result<String, RubyExecError> {
        self.get_config("LIBRUBYARG")
    }

    /// Returns the value of `RbConfig::CONFIG['LIBS']`.
    #[inline]
    pub fn libs(&self) -> Result<String, RubyExecError> {
        self.get_config("LIBS")
    }

    /// Returns the value of `RbConfig::CONFIG['MAINLIBS']`.
    #[inline]
    pub fn main_libs(&self) -> Result<String, RubyExecError> {
        self.get_config("MAINLIBS")
    }

    /// Returns the value of `RbConfig::CONFIG['SOLIBS']`.
    ///
    /// The returned value is a list of shared object libraries.
    #[inline]
    pub fn so_libs(&self) -> Result<String, RubyExecError> {
        self.get_config("SOLIBS")
    }

    /// The auxiliary libraries that should be dynamically linked to.
    #[inline]
    pub fn aux_libs(&self, static_lib: bool) -> Result<String, RubyExecError> {
        if static_lib {
            // Link to the same libraries as the main `ruby` program
            self.main_libs()
        } else {
            self.libs()
        }
    }

    /// Tells `cargo` to link to Ruby and its libraries.
    pub fn link(&self, static_lib: bool) -> Result<(), RubyLinkError> {
        link::link(self, static_lib)
    }

    /// Iterates over the header directory paths for the Ruby library.
    pub fn with_headers<F: FnMut(PathBuf)>(&self, mut f: F) -> io::Result<()> {
        util::walk_files(self.include_dir()?.as_ref(), |path| {
            if path.extension() == Some("h".as_ref()) {
                f(path);
            }
            Ok(())
        })
    }

    /// Returns all header paths for the Ruby library.
    pub fn headers(&self) -> io::Result<Vec<PathBuf>> {
        let mut headers = Vec::new();
        self.with_headers(|header| headers.push(header))?;
        Ok(headers)
    }

    /// Returns header contents with `#include`s that are suitable for passing
    /// into `bindgen`.
    ///
    /// This method filters out headers in `arch_header_dir`. If you'd like to
    /// keep those headers, use `wrapper_header_filtered` with a filter that
    /// returns `true`.
    pub fn wrapper_header(&self) -> io::Result<String> {
        let arch_header_dir = self.arch_header_dir()?;
        self.wrapper_header_filtered(|path| {
            !path.starts_with(&arch_header_dir)
        })
    }

    /// Returns header contents with filtered `#include`s that are suitable for
    /// passing into `bindgen`.
    ///
    /// Filtering of headers is left completely up to the caller. Note that
    /// headers in `arch_header_dir` will be passed in as well. This can
    /// sometimes lead to issues regarding redefined types.
    #[inline]
    pub fn wrapper_header_filtered<F>(&self, mut f: F) -> io::Result<String>
        where F: FnMut(&Path) -> bool,
    {
        self._wrapper_header_filtered(&mut f)
    }

    fn _wrapper_header_filtered(
        &self,
        f: &mut dyn FnMut(&Path) -> bool,
    ) -> io::Result<String> {
        let header_dir = self.header_dir()?;
        let header_dir = Path::new(&header_dir);

        let mut buf = String::new();

        // Workaround for `String` not implementing `io::Write`
        fn write_header(buf: &mut String, header: impl Display) -> io::Result<()> {
            use io::Write;
            let buf = unsafe { buf.as_mut_vec() };
            writeln!(buf, "#include <{}>", header)
        }

        util::walk_files(&header_dir, |path| {
            if path.extension() != Some("h".as_ref()) || !f(&path) {
                return Ok(());
            }
            match path.strip_prefix(header_dir) {
                Ok(header) => {
                    if cfg!(target_os = "windows") {
                        let header = header
                            .to_string_lossy()
                            .as_ref()
                            .replace('\\', "/");
                        write_header(&mut buf, header)?;
                    } else {
                        write_header(&mut buf, header.display())?;
                    }
                    Ok(())
                },
                Err(error) => {
                    Err(io::Error::new(io::ErrorKind::Other, error))
                },
            }
        })?;

        Ok(buf)
    }
}

/// The error returned when running `ruby` fails.
#[derive(Debug)]
pub enum RubyExecError {
    /// An I/O error occurred when executing `ruby`.
    ExecFail(io::Error),
    /// The `ruby` executable exited with a failure.
    RunFail(Output),
    /// The output of the config key is not encoded as UTF-8.
    Utf8Error(FromUtf8Error),
}

impl std::error::Error for RubyExecError {}

impl Display for RubyExecError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RubyExecError::ExecFail(error) => error.fmt(f),
            RubyExecError::RunFail(_) => {
                write!(f, "Failed to execute `ruby`")
            },
            RubyExecError::Utf8Error(error) => error.fmt(f),
        }
    }
}

impl From<io::Error> for RubyExecError {
    #[inline]
    fn from(error: io::Error) -> Self {
        RubyExecError::ExecFail(error)
    }
}

impl From<RubyExecError> for io::Error {
    #[inline]
    fn from(error: RubyExecError) -> Self {
        match error {
            RubyExecError::ExecFail(error) => error,
            error => io::Error::new(io::ErrorKind::Other, error)
        }
    }
}

impl From<FromUtf8Error> for RubyExecError {
    #[inline]
    fn from(error: FromUtf8Error) -> Self {
        RubyExecError::Utf8Error(error)
    }
}

impl RubyExecError {
    pub(crate) fn process(command: &mut Command) -> Result<String, Self> {
        let output = command.output()?;
        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?)
        } else {
            Err(RubyExecError::RunFail(output))
        }
    }
}
