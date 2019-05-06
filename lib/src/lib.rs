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

extern crate bzip2;
extern crate cc;
extern crate dirs;
extern crate memchr;
extern crate tar;
extern crate walkdir;

#[cfg(feature = "ureq")]
extern crate ureq;

use std::ffi::OsStr;
use std::fmt::{self, Display};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::string::FromUtf8Error;

use walkdir::{WalkDir, DirEntry};

mod archive;
mod link;
pub mod src;
pub mod version;

use version::RubyVersionError;

#[doc(inline)]
pub use self::{
    archive::Archive,
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
    lib_path: PathBuf,
    bin_path: PathBuf,
}

impl Ruby {
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
        let lib_path = out_dir.join("lib");

        let mut bin_path = out_dir.join("bin");
        if cfg!(target_os = "windows") {
            bin_path.push("ruby.exe");
        } else {
            bin_path.push("ruby");
        }

        Ruby { version, out_dir, lib_path, bin_path }
    }

    /// Creates a new instance from the `ruby` executable.
    #[inline]
    pub fn from_bin(ruby: impl AsRef<OsStr>) -> Result<Ruby, RubyVersionError> {
        let ruby = ruby.as_ref();
        Ruby::from_path(RubyExecError::process(
            Command::new(ruby).args(&["-e", "print RbConfig::CONFIG['prefix']"])
        )?)
    }

    /// Creates a new instance, finding out the version by running the `ruby`
    /// executable in `out_dir`.
    pub fn from_path(out_dir: impl Into<PathBuf>) -> Result<Ruby, RubyVersionError> {
        let mut ruby = Ruby::new(Version::new(0, 0, 0), out_dir);
        ruby.version = Version::from_bin(&ruby.bin_path)?;
        Ok(ruby)
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

    /// Iterates over the header directory entries for the Ruby library.
    pub fn with_header_entries<F>(&self, mut f: F) -> io::Result<()>
        where F: FnMut(DirEntry) -> ()
    {
        for entry in WalkDir::new(self.include_dir()?) {
            let entry = entry?;
            if entry.path().extension() == Some("h".as_ref()) {
                f(entry);
            }
        }
        Ok(())
    }

    /// Iterates over the header directory paths for the Ruby library.
    pub fn with_headers<F: FnMut(PathBuf)>(&self, mut f: F) -> io::Result<()> {
        self.with_header_entries(|entry| f(entry.into_path()))
    }

    /// Returns all header paths for the Ruby library.
    pub fn headers(&self) -> io::Result<Vec<PathBuf>> {
        let mut headers = Vec::new();
        self.with_headers(|header| headers.push(header))?;
        Ok(headers)
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
