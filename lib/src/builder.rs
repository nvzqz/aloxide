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
    autoconf: Command,
    force_autoconf: bool,
    configure: Command,
    configure_path: PathBuf,
    force_configure: bool,
    make: Command,
    force_make: bool,
}

impl RubyBuilder {
    pub(crate) fn new(
        version: Version,
        src_dir: PathBuf,
        out_dir: PathBuf,
    ) -> Self {
        let configure_path = src_dir.join("configure");

        let mut configure = Command::new(&configure_path);
        configure.arg(format!("--prefix={}", out_dir.display()));

        let mut make = Command::new("make");
        make.arg("install");
        make.env("PREFIX", &out_dir);

        RubyBuilder {
            version,
            src_dir,
            out_dir,
            autoconf: Command::new("autoconf"),
            force_autoconf: false,
            configure,
            configure_path,
            force_configure: false,
            make,
            force_make: false,
        }
    }

    /// Run `autoconf`, even if `configure` already exists.
    #[inline]
    pub fn force_autoconf(mut self) -> Self {
        self.force_autoconf = true;
        self
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

    /// Run `configure`, even if `Makefile` already exists.
    #[inline]
    pub fn force_configure(mut self) -> Self {
        self.force_configure = true;
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

    /// Run `make`, even if `out_dir/bin/ruby` already exists.
    #[inline]
    pub fn force_make(mut self) -> Self {
        self.force_make = true;
        self
    }

    /// Pass `args` into `make install`.
    #[inline]
    pub fn make_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: AsRef<OsStr>,
    {
        self.make.args(args);
        self
    }

    /// Sets the `stdin` handle of `make install`.
    #[inline]
    pub fn make_stdin<A: Into<Stdio>>(mut self, stdin: A) -> Self {
        self.make.stdin(stdin);
        self
    }

    /// Sets the `stdout` handle of `make install`.
    #[inline]
    pub fn make_stdout<A: Into<Stdio>>(mut self, stdout: A) -> Self {
        self.make.stdout(stdout);
        self
    }

    /// Sets the `stderr` handle of `make install`.
    #[inline]
    pub fn make_stderr<A: Into<Stdio>>(mut self, stderr: A) -> Self {
        self.make.stderr(stderr);
        self
    }

    /// Performs the required build steps for Ruby in one go.
    pub fn build(mut self) -> Result<Ruby, RubyBuildError> {
        use RubyBuildError::*;

        macro_rules! phase {
            ($cmd:ident, $cond:expr, $fail:ident, $spawn_fail:ident) => (
                if $cond {
                    match self.$cmd.current_dir(&self.src_dir).status() {
                        Ok(status) => if !status.success() {
                            return Err($fail(status));
                        },
                        Err(error) => {
                            return Err($spawn_fail(error));
                        },
                    }
                }
            )
        }

        let run_autoconf = self.force_autoconf || !self.configure_path.exists();
        phase!(autoconf, run_autoconf, AutoconfFail, AutoconfSpawnFail);

        let run_configure = run_autoconf || self.force_configure || !self.src_dir.join("Makefile").exists();
        phase!(configure, run_configure, ConfigureFail, ConfigureSpawnFail);

        let bin_path = self.out_dir.join("bin").join("ruby");
        let run_make = run_configure || self.force_make || !bin_path.exists();
        phase!(make, run_make, MakeFail, MakeSpawnFail);

        Ok(Ruby {
            version: self.version,
            src_dir: self.src_dir,
            out_dir: self.out_dir,
            bin_path,
        })
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

    /// Failed to spawn a process for `make`.
    MakeSpawnFail(io::Error),

    /// `make` exited unsuccessfully.
    MakeFail(ExitStatus),
}
