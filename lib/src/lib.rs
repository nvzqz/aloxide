//! Utilities for downloading, configuring, compiling, and installing Ruby.

#![deny(missing_docs)]

extern crate bzip2;
extern crate cc;
extern crate dirs;
extern crate http_req;
extern crate memchr;
extern crate tar;

use std::ffi::OsStr;
use std::fmt::Display;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::string::FromUtf8Error;

mod util;
pub mod build;
pub mod download;
pub mod version;

use self::{
    build::RubyBuildError,
    download::RubySrcDownloadError,
};

#[doc(inline)]
pub use self::{
    build::RubyBuilder,
    download::RubySrcDownloader,
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
    pub fn src_downloader<'a, P: AsRef<Path> + ?Sized>(
        version: Version,
        dst_dir: &'a P,
    ) -> RubySrcDownloader<'a> {
        RubySrcDownloader::new(version, dst_dir.as_ref())
    }

    /// Downloads and unpacks the source for `version` to `dst_dir` with the
    /// default configuration.
    #[inline]
    pub fn download_src(
        version: Version,
        dst_dir: impl AsRef<Path>,
    ) -> Result<PathBuf, RubySrcDownloadError> {
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

    // FIXME: This leads to linking issues where external symbols prefixed with
    // `__imp__` cannot be found.
    #[cfg(windows)]
    fn _link_imp(&self, lib_args: String, kind: &str) -> Result<(), RubyLinkError> {
        let mut linked_ruby = false;

        for lib in lib_args.split_whitespace() {
            if lib.contains("ruby") {
                linked_ruby = true;
            }
            let name = &lib[..(lib.len() - 4)];
            println!("cargo:rustc-link-lib={}={}", kind, name);
        }

        if linked_ruby {
            Ok(())
        } else {
            Err(RubyLinkError::MissingRuby(lib_args))
        }
    }

    #[cfg(not(windows))]
    fn _link_imp(&self, lib_args: String, kind: &str) -> Result<(), RubyLinkError> {
        let mut iter = lib_args.split_whitespace();
        let mut linked_ruby = false;

        while let Some(flag) = iter.next() {
            if flag == "-framework" {
                if let Some(framework) = iter.next() {
                    println!("cargo:rustc-link-lib=framework={}", framework);
                } else {
                    return Err(RubyLinkError::MissingFramework(lib_args));
                }
            } else if flag.starts_with("-l") {
                let name = &flag[2..];
                if name.starts_with("ruby") {
                    linked_ruby = true;
                    println!("cargo:rustc-link-lib={}={}", kind, name);
                } else {
                    println!("cargo:rustc-link-lib={}", name);
                }
            }
        }

        if linked_ruby {
            Ok(())
        } else {
            Err(RubyLinkError::MissingRuby(lib_args))
        }
    }

    /// Tells `cargo` to link to Ruby and its libraries.
    pub fn link(&self, static_lib: bool) -> Result<(), RubyLinkError> {
        println!("cargo:rustc-link-search={}", self.lib_path.display());

        let (key, kind) = if static_lib {
            ("LIBRUBYARG_STATIC", "static")
        } else {
            ("LIBRUBYARG_SHARED", "dylib")
        };
        let lib_args = self.get_config(key).map_err(RubyLinkError::Exec)?;

        self._link_imp(lib_args, kind)
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

/// The error returned when linking to the Ruby library and its dependencies
/// fails.
#[derive(Debug)]
pub enum RubyLinkError {
    /// Failed to execute the `ruby` binary.
    Exec(RubyExecError),
    /// Did not link to the Ruby library.
    MissingRuby(String),
    /// A `-framework` flag was found with no argument.
    MissingFramework(String),
}
