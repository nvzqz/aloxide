//! Utilities for downloading, configuring, compiling, and installing Ruby.

#![deny(missing_docs)]

extern crate memchr;

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
pub struct Ruby {
    version: Version,
    src_dir: PathBuf,
    out_dir: PathBuf,
}

impl Ruby {
    /// Returns a new Ruby builder.
    #[inline]
    pub fn builder(
        version: Version,
        src_dir: impl Into<PathBuf>,
        out_dir: impl Into<PathBuf>,
    ) -> RubyBuilder {
        RubyBuilder::new(version, src_dir.into(), out_dir.into())
    }

    /// Builds Ruby with the default configuration.
    #[inline]
    pub fn build(
        version: Version,
        src_dir: impl Into<PathBuf>,
        out_dir: impl Into<PathBuf>,
    ) -> Result<Self, RubyBuildError> {
        Self::builder(version, src_dir, out_dir).build()
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
}
