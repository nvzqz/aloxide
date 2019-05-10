//! Utilities for Ruby's source code.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Version;

pub mod build;

#[cfg(feature = "download")]
pub mod download;

#[doc(inline)]
pub use build::RubyBuilder;

#[cfg(feature = "download")]
#[doc(inline)]
pub use download::RubySrcDownloader;

/// A path to Ruby's source code.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RubySrc(Path);

impl AsRef<Path> for RubySrc {
    #[inline]
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl From<Box<Path>> for Box<RubySrc> {
    #[inline]
    fn from(dir: Box<Path>) -> Self {
        unsafe { Box::from_raw(Box::into_raw(dir) as *mut RubySrc) }
    }
}

impl From<PathBuf> for Box<RubySrc> {
    #[inline]
    fn from(dir: PathBuf) -> Self {
        dir.into_boxed_path().into()
    }
}

impl From<Box<RubySrc>> for Box<Path> {
    #[inline]
    fn from(src: Box<RubySrc>) -> Self {
        unsafe { Box::from_raw(Box::into_raw(src) as *mut Path) }
    }
}

impl From<Box<RubySrc>> for PathBuf {
    #[inline]
    fn from(src: Box<RubySrc>) -> Self {
        src.into_path().into()
    }
}

impl RubySrc {
    /// Creates a new instance targeting `dir`.
    #[inline]
    pub fn new<P: AsRef<Path> + ?Sized>(dir: &P) -> &Self {
        unsafe { &*(dir.as_ref() as *const Path as *const Self) }
    }

    /// Returns a downloader for `version` targeted towards `self`.
    #[inline]
    #[cfg(feature = "download")]
    pub fn downloader<'a, P: AsRef<Path> + ?Sized>(
        version: &'a Version,
        parent: &'a P,
    ) -> RubySrcDownloader<'a> {
        RubySrcDownloader::new(version, parent.as_ref())
    }

    /// Returns the directory path.
    #[inline]
    pub fn as_path(&self) -> &Path {
        self.as_ref()
    }

    /// Converts `self` into a `Path`.
    #[inline]
    pub fn into_path(self: Box<Self>) -> Box<Path> {
        self.into()
    }

    /// Converts `self` into a `PathBuf`.
    #[inline]
    pub fn into_path_buf(self: Box<Self>) -> PathBuf {
        self.into()
    }

    /// Creates a new builder for Ruby's sources.
    #[inline]
    pub fn builder<'a>(
        &'a self,
        out_dir: impl Into<PathBuf>,
        target: impl AsRef<str>,
    ) -> RubyBuilder<'a> {
        RubyBuilder::new(self, out_dir.into(), target.as_ref())
    }

    /// Returns a `make` command suitable for `target` to run in this directory.
    #[inline]
    pub fn make(&self, target: impl AsRef<str>) -> Command {
        let mut cmd = match crate::util::nmake(target.as_ref()) {
            Some(cmd) => cmd,
            None => Command::new("make"),
        };
        cmd.current_dir(self);
        cmd
    }
}
