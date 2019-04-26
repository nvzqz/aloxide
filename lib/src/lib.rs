//! Utilities for downloading, configuring, compiling, and installing Ruby.

#![deny(missing_docs)]

extern crate memchr;

use std::ffi::OsStr;
use std::fmt::Display;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::string::FromUtf8Error;

mod version;
mod builder;

pub use self::{
    builder::*,
    version::*,
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
    bin_path: PathBuf,
}

impl Ruby {
    /// Returns a new Ruby builder.
    #[inline]
    pub fn builder(
        src_dir: impl Into<PathBuf>,
        out_dir: impl Into<PathBuf>,
    ) -> RubyBuilder {
        RubyBuilder::new(src_dir.into(), out_dir.into())
    }

    /// Builds Ruby with the default configuration.
    #[inline]
    pub fn build(
        src_dir: impl Into<PathBuf>,
        out_dir: impl Into<PathBuf>,
    ) -> Result<Self, RubyBuildError> {
        Self::builder(src_dir, out_dir).build()
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
        let bin_path = out_dir.join("bin").join("ruby");
        Ruby { version, src_dir, out_dir, bin_path }
    }

    /// Returns the Ruby version.
    #[inline]
    pub fn version(&self) -> Version {
        self.version
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

    /// Executes `ruby_script` through the interpreter at `bin_path`.
    pub fn run(&self, ruby_script: impl AsRef<OsStr>) -> io::Result<Output> {
        Command::new(&self.bin_path)
            .args(&["-e".as_ref(), ruby_script.as_ref()])
            .output()
    }

    /// Returns the configuration value for `key`.
    pub fn get_config(&self, key: impl Display) -> Result<String, RubyGetConfigError> {
        let output = self.run(&format!("print RbConfig::CONFIG['{}']", key))?;
        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?)
        } else {
            Err(RubyGetConfigError::RunFail(output))
        }
    }
}

/// The error returned when
/// [`Ruby::get_config`](struct.Ruby.html#method.get_config) fails.
#[derive(Debug)]
pub enum RubyGetConfigError {
    /// An IO error occurred when executing `ruby`.
    Io(io::Error),
    /// The `ruby` executable exited with a failure.
    RunFail(Output),
    /// The output of the config key is not encoded as UTF-8.
    Utf8Error(FromUtf8Error),
}

impl From<io::Error> for RubyGetConfigError {
    #[inline]
    fn from(error: io::Error) -> Self {
        RubyGetConfigError::Io(error)
    }
}

impl From<FromUtf8Error> for RubyGetConfigError {
    #[inline]
    fn from(error: FromUtf8Error) -> Self {
        RubyGetConfigError::Utf8Error(error)
    }
}
