//! Utilities for building Ruby.

use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use crate::{Ruby, version::{Version, VersionParseError}};

/// Configures and builds Ruby.
pub struct RubyBuilder {
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
        src_dir: PathBuf,
        out_dir: PathBuf,
        target: &str,
    ) -> Self {
        let configure_path = if cfg!(target_os = "windows") {
            let mut path = src_dir.join("win32");
            path.push("configure.bat");
            path
        } else {
            src_dir.join("configure")
        };

        let mut configure = Command::new(&configure_path);
        configure.arg(format!("--prefix={}", out_dir.display()));

        let mut make = match cc::windows_registry::find(target, "nmake.exe") {
            Some(nmake) => nmake,
            None => Command::new("make"),
        };
        make.arg("install");
        make.env("PREFIX", &out_dir);

        RubyBuilder {
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

    /// Adjust what happens when running `autoconf`.
    #[inline]
    pub fn autoconf(self) -> AutoconfPhase {
        AutoconfPhase(self)
    }

    /// Adjust what happens when running `configure`.
    #[inline]
    pub fn configure(self) -> ConfigurePhase {
        ConfigurePhase(self)
    }

    /// Adjust what happens when running `make`.
    #[inline]
    pub fn make(self) -> MakePhase {
        MakePhase(self)
    }

    /// Performs the required build steps for Ruby in one go.
    pub fn build(mut self) -> Result<Ruby, RubyBuildError> {
        use RubyBuildError::*;

        macro_rules! phase {
            ($cmd:ident, $cond:expr, $fail:ident, $spawn_fail:ident) => (
                if $cond {
                    match self.$cmd.current_dir(&self.src_dir).output() {
                        Ok(output) => if !output.status.success() {
                            return Err($fail(output));
                        },
                        Err(error) => {
                            return Err($spawn_fail(error));
                        },
                    }
                }
            )
        }

        let run_autoconf = if cfg!(target_os = "windows") {
            false
        } else {
            let run_autoconf = self.force_autoconf || !self.configure_path.exists();
            phase!(autoconf, run_autoconf, AutoconfFail, AutoconfSpawnFail);
            run_autoconf
        };

        let run_configure = run_autoconf || self.force_configure || !self.src_dir.join("Makefile").exists();
        phase!(configure, run_configure, ConfigureFail, ConfigureSpawnFail);

        let mut bin_path = self.out_dir.join("bin");
        if cfg!(target_os = "windows") {
            bin_path.push("ruby.exe");
        } else {
            bin_path.push("ruby")
        }

        let run_make = run_configure || self.force_make || !bin_path.exists();
        phase!(make, run_make, MakeFail, MakeSpawnFail);

        let mut ruby_version = Command::new(&bin_path);
        ruby_version.args(&["-e", "print RbConfig::CONFIG['RUBY_PROGRAM_VERSION']"]);

        let version = match ruby_version.output() {
            Ok(output) => match String::from_utf8(output.stdout) {
                Ok(utf8) => match utf8.parse::<Version>() {
                    Ok(version) => version,
                    Err(error) => return Err(RubyVersionParseFail(error)),
                },
                Err(error) => return Err(RubyVersionUtf8Fail(error)),
            },
            Err(error) => return Err(RubySpawnFail(error)),
        };

        let lib_path = self.out_dir.join("lib");
        Ok(Ruby {
            version,
            src_dir: self.src_dir,
            out_dir: self.out_dir,
            lib_path,
            bin_path,
        })
    }
}

/// Adjusts what happens when running `autoconf`.
///
/// **Note:** On the MSVC target platform, `autoconf` is not run.
pub struct AutoconfPhase(RubyBuilder);

impl AutoconfPhase {
    /// Force `autoconf` to run if applicable.
    #[inline]
    pub fn force(mut self) -> Self {
        self.0.force_autoconf = true;
        self
    }

    /// Pass `args` into `autoconf`.
    #[inline]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: AsRef<OsStr>,
    {
        self.0.autoconf.args(args);
        self
    }

    /// Pass the environment vars into `autoconf`.
    #[inline]
    pub fn envs<I, K, V>(mut self, envs: I) -> Self
    where
        I: IntoIterator<Item=(K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.0.autoconf.envs(envs);
        self
    }

    /// Remove the environment vars for `autoconf`.
    #[inline]
    pub fn remove_envs<I, S>(mut self, envs: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: AsRef<OsStr>,
    {
        for key in envs { self.0.autoconf.env_remove(key); }
        self
    }

    /// Sets the `stdin` handle of `autoconf`.
    #[inline]
    pub fn stdin<A: Into<Stdio>>(mut self, stdin: A) -> Self {
        self.0.autoconf.stdin(stdin);
        self
    }

    /// Sets the `stdout` handle of `autoconf`.
    #[inline]
    pub fn stdout<A: Into<Stdio>>(mut self, stdout: A) -> Self {
        self.0.autoconf.stdout(stdout);
        self
    }

    /// Sets the `stderr` handle of `autoconf`.
    #[inline]
    pub fn stderr<A: Into<Stdio>>(mut self, stderr: A) -> Self {
        self.0.autoconf.stderr(stderr);
        self
    }

    /// Adjust what happens when running `configure`.
    #[inline]
    pub fn configure(self) -> ConfigurePhase {
        ConfigurePhase(self.0)
    }

    /// Adjust what happens when running `make`.
    #[inline]
    pub fn make(self) -> MakePhase {
        MakePhase(self.0)
    }

    /// Perform the build.
    #[inline]
    pub fn build(self) -> Result<Ruby, RubyBuildError> {
        self.0.build()
    }
}

/// Adjusts what happens when running `configure`.
///
/// **Note:** On the MSVC target platform, `win32/configure.bat` is run instead
/// of `configure`.
pub struct ConfigurePhase(RubyBuilder);

impl ConfigurePhase {
    /// Force `configure` to run.
    #[inline]
    pub fn force(mut self) -> Self {
        self.0.force_configure = true;
        self
    }

    /// Pass `args` into `configure`.
    #[inline]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: AsRef<OsStr>,
    {
        self.0.configure.args(args);
        self
    }

    /// Pass the environment vars into `configure`.
    #[inline]
    pub fn envs<I, K, V>(mut self, envs: I) -> Self
    where
        I: IntoIterator<Item=(K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.0.configure.envs(envs);
        self
    }

    /// Remove the environment vars for `configure`.
    #[inline]
    pub fn remove_envs<I, S>(mut self, envs: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: AsRef<OsStr>,
    {
        for key in envs { self.0.configure.env_remove(key); }
        self
    }

    /// Sets the `stdin` handle of `configure`.
    #[inline]
    pub fn stdin<A: Into<Stdio>>(mut self, stdin: A) -> Self {
        self.0.configure.stdin(stdin);
        self
    }

    /// Sets the `stdout` handle of `configure`.
    #[inline]
    pub fn stdout<A: Into<Stdio>>(mut self, stdout: A) -> Self {
        self.0.configure.stdout(stdout);
        self
    }

    /// Sets the `stderr` handle of `configure`.
    #[inline]
    pub fn stderr<A: Into<Stdio>>(mut self, stderr: A) -> Self {
        self.0.configure.stderr(stderr);
        self
    }

    /// Adjust what happens when running `make`.
    #[inline]
    pub fn make(self) -> MakePhase {
        MakePhase(self.0)
    }

    /// Perform the build.
    #[inline]
    pub fn build(self) -> Result<Ruby, RubyBuildError> {
        self.0.build()
    }
}

/// Adjusts what happens when running `make install`.
///
/// **Note:** On the MSVC target platform, `nmake` is used instead of `make`.
pub struct MakePhase(RubyBuilder);

impl MakePhase {
    /// Force `make install` to run.
    #[inline]
    pub fn force(mut self) -> Self {
        self.0.force_make = true;
        self
    }

    /// Pass `args` into `make install`.
    #[inline]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: AsRef<OsStr>,
    {
        self.0.make.args(args);
        self
    }

    /// Pass the environment vars into `make install`.
    #[inline]
    pub fn envs<I, K, V>(mut self, envs: I) -> Self
    where
        I: IntoIterator<Item=(K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.0.make.envs(envs);
        self
    }

    /// Remove the environment vars for `make install`.
    #[inline]
    pub fn remove_envs<I, S>(mut self, envs: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: AsRef<OsStr>,
    {
        for key in envs { self.0.make.env_remove(key); }
        self
    }

    /// Sets the `stdin` handle of `make install`.
    #[inline]
    pub fn stdin<A: Into<Stdio>>(mut self, stdin: A) -> Self {
        self.0.make.stdin(stdin);
        self
    }

    /// Sets the `stdout` handle of `make install`.
    #[inline]
    pub fn stdout<A: Into<Stdio>>(mut self, stdout: A) -> Self {
        self.0.make.stdout(stdout);
        self
    }

    /// Sets the `stderr` handle of `make install`.
    #[inline]
    pub fn stderr<A: Into<Stdio>>(mut self, stderr: A) -> Self {
        self.0.make.stderr(stderr);
        self
    }

    /// Perform the build.
    #[inline]
    pub fn build(self) -> Result<Ruby, RubyBuildError> {
        self.0.build()
    }
}

/// The error returned when
/// [`RubyBuilder::build`](struct.RubyBuilder.html#method.build) fails.
#[derive(Debug)]
pub enum RubyBuildError {
    /// Failed to spawn a process for `autoconf`.
    AutoconfSpawnFail(io::Error),
    /// `autoconf` exited unsuccessfully.
    AutoconfFail(Output),
    /// Failed to spawn a process for `configure`.
    ConfigureSpawnFail(io::Error),
    /// `configure` exited unsuccessfully.
    ConfigureFail(Output),
    /// Failed to spawn a process for `make`.
    MakeSpawnFail(io::Error),
    /// `make` exited unsuccessfully.
    MakeFail(Output),
    /// Failed to spawn a process for `ruby`.
    RubySpawnFail(io::Error),
    /// Failed to parse the Ruby version as UTF-8.
    RubyVersionUtf8Fail(std::string::FromUtf8Error),
    /// Failed to parse the Ruby version as a `Version`.
    RubyVersionParseFail(VersionParseError),
}
