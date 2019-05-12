//! Utilities for building Ruby.

use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::borrow::Borrow;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use crate::{Ruby, RubySrc, version::RubyVersionError};

/// Configures and builds Ruby.
pub struct RubyBuilder<'a> {
    src: &'a RubySrc,
    out_dir: PathBuf,
    autoconf: Command,
    force_autoconf: bool,
    configure: Command,
    configure_path: PathBuf,
    force_configure: bool,
    make: Command,
    force_make: bool,

    #[cfg(windows)]
    target_msvc: bool,
}

impl<'a> RubyBuilder<'a> {
    // Process `target` to make it usable with building for Windows
    fn convert_to_ruby(target: &str) -> &str {
        match target {
            "x86_64-pc-windows-msvc" => "x84_64-mswin64",
            "x86_64-pc-windows-gnu"  => "x86_64-pc-mingw32",
            "i686-pc-windows-msvc"   => "x86-mswin32",
            "i686-pc-windows-gnu"    => "x86-pc-mingw32",
            other => other
        }
    }

    // Process `target` to make it usable with `cc::windows_registry::find`
    fn convert_to_rust(target: &str) -> &str {
        if target.contains("mswin") {
            if target.contains("64") {
                "x86_64-pc-windows-msvc"
            } else {
                "i686-pc-windows-msvc"
            }
        } else {
            target
        }
    }

    pub(crate) fn new(
        src: &'a RubySrc,
        out_dir: PathBuf,
        target: &str,
    ) -> Self {
        let src_dir = src.as_path();
        let ruby_target = RubyBuilder::convert_to_ruby(target);
        let rust_target = RubyBuilder::convert_to_rust(target);

        let nmake = crate::util::nmake(rust_target);
        let target_msvc = cfg!(target_os = "windows") && nmake.is_some();

        let (mut make, configure_path) = match nmake {
            Some(nmake) => {
                let mut path = src_dir.join("win32");
                path.push("configure.bat");
                (nmake, path)
            },
            None => (Command::new("make"), src_dir.join("configure"))
        };

        make.arg("install");
        make.env("PREFIX", &out_dir);

        let mut configure = if cfg!(target_os = "windows") && !target_msvc {
            // HACK: Spawn `configure` via `sh` since `Command::new` requires a
            // Win32 application to work
            let mut sh = Command::new("sh.exe");
            sh.arg("configure");
            sh
        } else {
            Command::new(&configure_path)
        };

        configure.arg(format!("--prefix={}", out_dir.display()));
        configure.arg(format!("--target={}", ruby_target));

        RubyBuilder {
            src,
            out_dir,
            autoconf: Command::new("autoconf"),
            force_autoconf: false,
            configure,
            configure_path,
            force_configure: false,
            make,
            force_make: false,

            #[cfg(windows)]
            target_msvc,
        }
    }

    /// Adjust what happens when running `autoconf`.
    #[inline]
    pub fn autoconf(self) -> AutoconfPhase<'a> {
        AutoconfPhase(self)
    }

    /// Adjust what happens when running `configure`.
    #[inline]
    pub fn configure(self) -> ConfigurePhase<'a> {
        ConfigurePhase(self)
    }

    /// Adjust what happens when running `make`.
    #[inline]
    pub fn make(self) -> MakePhase<'a> {
        MakePhase(self)
    }

    /// Performs the required build steps for Ruby in one go.
    pub fn build(mut self) -> Result<Ruby, RubyBuildError> {
        use RubyBuildError::*;
        use crate::Version;

        #[cfg(target_os = "windows")]
        let target_msvc = self.target_msvc;

        #[cfg(not(target_os = "windows"))]
        let target_msvc = false;

        macro_rules! phase {
            ($cmd:ident, $cond:expr, $fail:ident, $spawn_fail:ident) => (
                if $cond {
                    let output = self.$cmd
                        .current_dir(&self.src)
                        .output()
                        .map_err($spawn_fail)?;

                    if !output.status.success() {
                        return Err($fail(output));
                    }
                }
            )
        }

        let run_autoconf = if target_msvc {
            false
        } else {
            let run_autoconf = self.force_autoconf || !self.configure_path.exists();
            phase!(autoconf, run_autoconf, AutoconfFail, AutoconfSpawnFail);
            run_autoconf
        };

        let src_dir = self.src.as_path();

        let run_configure = run_autoconf || self.force_configure || !src_dir.join("Makefile").exists();
        phase!(configure, run_configure, ConfigureFail, ConfigureSpawnFail);

        let bin_path = self.out_dir.join("bin").join(Ruby::bin_name());

        let run_make = run_configure || self.force_make || !bin_path.exists();
        phase!(make, run_make, MakeFail, MakeSpawnFail);

        let lib_dir = self.out_dir.join("lib");
        Ok(Ruby {
            version: Version::from_bin(&bin_path)?,
            out_dir: self.out_dir,
            lib_dir,
            bin_path,
        })
    }
}

/// Adjusts what happens when running `autoconf`.
///
/// **Note:** On the MSVC target platform, `autoconf` is not run.
pub struct AutoconfPhase<'a>(RubyBuilder<'a>);

impl<'a> AutoconfPhase<'a> {
    /// Force `autoconf` to run if applicable.
    #[inline]
    pub fn force(mut self) -> Self {
        self.0.force_autoconf = true;
        self
    }

    /// Perform custom operations on the `Command` instance used.
    #[inline]
    pub fn with_command<F: FnOnce(&mut Command) -> ()>(mut self, f: F) -> Self {
        f(&mut self.0.autoconf);
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
    pub fn configure(self) -> ConfigurePhase<'a> {
        ConfigurePhase(self.0)
    }

    /// Adjust what happens when running `make`.
    #[inline]
    pub fn make(self) -> MakePhase<'a> {
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
pub struct ConfigurePhase<'a>(RubyBuilder<'a>);

impl<'a> ConfigurePhase<'a> {
    /// Force `configure` to run.
    #[inline]
    pub fn force(mut self) -> Self {
        self.0.force_configure = true;
        self
    }

    /// Sets the value for `key` to `val`.
    #[inline]
    pub fn set_val(
        mut self,
        key: impl Into<OsString>,
        val: impl AsRef<OsStr>,
    ) -> Self {
        let mut arg = key.into();
        arg.push("=");
        arg.push(val);
        self.0.configure.arg(arg);
        self
    }

    /// Inherits the value for the environment variable `env`.
    #[inline]
    pub fn inherit_env(self, env: impl AsRef<OsStr>) -> Self {
        let env = env.as_ref();
        if let Some(var) = std::env::var_os(env) {
            self.set_val(env, var)
        } else {
            self
        }
    }

    /// Sets the C compiler that Ruby should use.
    #[inline]
    pub fn set_cc(self, cc: impl AsRef<OsStr>) -> Self {
        self.set_val("CC", cc)
    }

    /// Sets whether Ruby should use the C compiler defined by the `CC`
    /// environment variable.
    #[inline]
    pub fn inherit_cc(self) -> Self {
        self.inherit_env("CC")
    }

    /// Sets the flags for the C compiler.
    #[inline]
    pub fn set_c_flags(self, flags: impl AsRef<OsStr>) -> Self {
        self.set_val("CFLAGS", flags)
    }

    /// Sets the flags for the C compiler defined by the `CFLAGS` environment
    /// variable.
    #[inline]
    pub fn inherit_c_flags(self) -> Self {
        self.inherit_env("CFLAGS")
    }

    /// Include `feature`.
    #[inline]
    pub fn enable(mut self, feature: impl Display) -> Self {
        self.0.configure.arg(format!("--enable-{}", feature));
        self
    }

    /// Disable `feature`.
    #[inline]
    pub fn disable(mut self, feature: impl Display) -> Self {
        self.0.configure.arg(format!("--disable-{}", feature));
        self
    }

    /// Include `package`.
    #[inline]
    pub fn with(mut self, package: impl Display) -> Self {
        self.0.configure.arg(format!("--with-{}", package));
        self
    }

    /// Remove `package`.
    #[inline]
    pub fn without(mut self, package: impl Display) -> Self {
        self.0.configure.arg(format!("--without-{}", package));
        self
    }

    /// Whether to build a shared library for Ruby.
    ///
    /// The default value is `false`.
    #[inline]
    pub fn shared_lib(mut self, enable_shared: bool) -> Self {
        let flag = if enable_shared {
            "--enable-shared"
        } else {
            "--disable-shared"
        };
        self.0.configure.arg(flag);
        self
    }

    /// Whether to build a static library for Ruby.
    ///
    /// The default value is `true`.
    #[inline]
    pub fn static_lib(mut self, enable_static: bool) -> Self {
        let flag = if enable_static {
            "--enable-install-static-library"
        } else {
            "--disable-install-static-library"
        };
        self.0.configure.arg(flag);
        self
    }

    /// Build an Apple/NeXT Multi Architecture Binary (MAB). If this option is
    /// disabled or omitted entirely, then the package will be built only for
    /// the target platform.
    ///
    /// Passes `archs` as comma-separated values into `--with-arch=`.
    #[inline]
    pub fn arch(mut self, archs: &[impl Borrow<str>]) -> Self {
        self.0.configure.arg(format!("--with-arch={}", archs.join(",")));
        self
    }

    /// Do not install neither rdoc indexes nor C API documents during install.
    #[inline]
    pub fn disable_install_doc(mut self) -> Self {
        self.0.configure.arg("--disable-install-doc");
        self
    }

    /// Disable dynamic link feature.
    #[inline]
    pub fn disable_dy_link(mut self) -> Self {
        self.0.configure.arg("--disable-dln");
        self
    }

    /// Resolve load paths at run time.
    #[inline]
    pub fn enable_load_relative(mut self) -> Self {
        self.0.configure.arg("--enable-load-relative");
        self
    }

    /// Disable rubygems by default.
    #[inline]
    pub fn disable_rubygems(mut self) -> Self {
        self.0.configure.arg("--disable-rubygems");
        self
    }

    /// Perform custom operations on the `Command` instance used.
    #[inline]
    pub fn with_command<F: FnOnce(&mut Command) -> ()>(mut self, f: F) -> Self {
        f(&mut self.0.configure);
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
    pub fn make(self) -> MakePhase<'a> {
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
pub struct MakePhase<'a>(RubyBuilder<'a>);

impl MakePhase<'_> {
    /// Force `make install` to run.
    #[inline]
    pub fn force(mut self) -> Self {
        self.0.force_make = true;
        self
    }

    /// Perform custom operations on the `Command` instance used.
    #[inline]
    pub fn with_command<F: FnOnce(&mut Command) -> ()>(mut self, f: F) -> Self {
        f(&mut self.0.make);
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
    /// Failed to get the version for `ruby`.
    Version(RubyVersionError),
}

impl From<RubyVersionError> for RubyBuildError {
    #[inline]
    fn from(error: RubyVersionError) -> Self {
        RubyBuildError::Version(error)
    }
}
