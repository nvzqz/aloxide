use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};

use crate::{Ruby, Version};

/// Configures and builds Ruby.
pub struct RubyBuilder {
    version: Version,
    src_dir: PathBuf,
    out_dir: PathBuf,
    configure: Command,
    configure_path: PathBuf,
    autoconf: Command,
    force_autoconf: bool,
}

impl RubyBuilder {
    pub(crate) fn new(
        version: Version,
        src_dir: PathBuf,
        out_dir: PathBuf
    ) -> Self {
        let configure_path = src_dir.join("configure");
        RubyBuilder {
            version,
            src_dir,
            out_dir,
            configure: Command::new(&configure_path),
            configure_path,
            autoconf: Command::new("autoconf"),
            force_autoconf: false,
        }
    }

    /// Pass `args` into `autoconf` when generating `configure`.
    #[inline]
    pub fn autoconf_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: AsRef<OsStr>,
    {
        self.autoconf.args(args);
        self
    }

    /// Sets the `stdin` handle of `autoconf`.
    #[inline]
    pub fn autoconf_stdin<A: Into<Stdio>>(mut self, stdin: A) -> Self {
        self.autoconf.stdin(stdin);
        self
    }

    /// Sets the `stdout` handle of `autoconf`.
    #[inline]
    pub fn autoconf_stdout<A: Into<Stdio>>(mut self, stdout: A) -> Self {
        self.autoconf.stdout(stdout);
        self
    }

    /// Sets the `stderr` handle of `autoconf`.
    #[inline]
    pub fn autoconf_stderr<A: Into<Stdio>>(mut self, stderr: A) -> Self {
        self.autoconf.stderr(stderr);
        self
    }

    /// Run `autoconf`, even if `configure` already exists.
    #[inline]
    pub fn force_autoconf(mut self) -> Self {
        self.force_autoconf = true;
        self
    }

    /// Pass `args` into the `configure` script.
    #[inline]
    pub fn configure_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: AsRef<OsStr>,
    {
        self.configure.args(args);
        self
    }

    /// Sets the `stdin` handle of `configure`.
    #[inline]
    pub fn configure_stdin<A: Into<Stdio>>(mut self, stdin: A) -> Self {
        self.configure.stdin(stdin);
        self
    }

    /// Sets the `stdout` handle of `configure`.
    #[inline]
    pub fn configure_stdout<A: Into<Stdio>>(mut self, stdout: A) -> Self {
        self.configure.stdout(stdout);
        self
    }

    /// Sets the `stderr` handle of `configure`.
    #[inline]
    pub fn configure_stderr<A: Into<Stdio>>(mut self, stderr: A) -> Self {
        self.configure.stderr(stderr);
        self
    }

    /// Performs all of the build steps for Ruby in one go.
    pub fn build(mut self) -> Result<Ruby, RubyBuildError> {
        use RubyBuildError::*;

        if self.force_autoconf || !self.configure_path.exists() {
            match self.autoconf.current_dir(&self.src_dir).status() {
                Ok(status) => if !status.success() {
                    return Err(AutoconfFail(status));
                },
                Err(error) => {
                    return Err(AutoconfSpawnFail(error));
                },
            }
        }

        match self.configure.status() {
            Ok(status) => if !status.success() {
                return Err(ConfigureFail(status));
            },
            Err(error) => {
                return Err(ConfigureSpawnFail(error));
            },
        }

        Ok(Ruby::new(self.version, self.src_dir, self.out_dir))
    }
}

/// The error returned when
/// [`RubyBuilder::build`](struct.RubyBuilder.html#method.build) fails.
#[derive(Debug)]
pub enum RubyBuildError {
    /// Failed to spawn a process for `autoconf`.
    AutoconfSpawnFail(io::Error),

    /// `autoconf` exited unsuccessfully.
    AutoconfFail(ExitStatus),

    /// Failed to spawn a process for `configure`.
    ConfigureSpawnFail(io::Error),

    /// `configure` exited unsuccessfully.
    ConfigureFail(ExitStatus),
}
