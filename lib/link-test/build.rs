extern crate aloxide;

use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use aloxide::{Ruby, RubySrc, Version};

// An external driver that manages the Ruby installation
enum Driver {
    // https://github.com/rvm/rvm
    Rvm,
    // https://www.github.com/rbenv/rbenv
    Rbenv,
}

impl Driver {
    fn get() -> Option<Driver> {
        if has_env("ALOXIDE_USE_RVM") {
            Some(Driver::Rvm)
        } else if has_env("ALOXIDE_USE_RBENV") {
            Some(Driver::Rbenv)
        } else {
            None
        }
    }

    fn ruby(self, version: &Version) -> Ruby {
        match self {
            Driver::Rvm => {
                Ruby::from_cmd(Command::new("rvm")
                    .arg(version.to_string())
                    .arg("do")
                    .arg("ruby")).expect("Could not execute `rvm`")
            },
            Driver::Rbenv => {
                Ruby::from_cmd(Command::new("rbenv")
                    .env("RBENV_VERSION", version.to_string())
                    .arg("exec")
                    .arg("ruby")).expect("Could not execute `rbenv`")
            },
        }
    }
}

fn build_ruby(version: &Version, static_lib: bool) -> Ruby {
    println!("Building Ruby {}", version);

    let target = env::var("TARGET").unwrap();

    let aloxide = match env::var_os("ALOXIDE_TEST_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => {
            match env::var_os("CARGO_TARGET_DIR") {
                Some(dir) => PathBuf::from(dir),
                None => {
                    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
                        .parent().unwrap()
                        .parent().unwrap()
                        .join("target")
                }
            }.join("aloxide")
        }
    };

    let target_dir = aloxide.join(&target);
    let out_dir = target_dir.join(&format!("ruby-{}-out", version));

    let cache = env::var_os("ALOXIDE_RUBY_CACHE");
    let mut downloader = RubySrc::downloader(version, &target_dir);
    if let Some(cache) = &cache {
        downloader = downloader.cache_dir(cache);
    }

    downloader
        .cache()
        .download()
        .expect(&format!("Failed to download Ruby {}", version))
        .builder(out_dir, target)
        .autoconf()
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
        .configure()
            .inherit_cc()
            .inherit_c_flags()
            .shared_lib(!static_lib)
            .disable_install_doc()
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
        .make()
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
        .build()
        .expect(&format!("Failed to build Ruby {}", version))
}

fn has_env(key: impl AsRef<OsStr>) -> bool {
    env::var_os(key).map(|var| !var.is_empty()).unwrap_or(false)
}

fn config(ruby: &Ruby) -> String {
    ruby.run("require 'pp'; pp RbConfig::CONFIG").expect("Failed to get config")
}

fn rerun_if_env_changed(var: &str) {
    println!("cargo:rerun-if-env-changed={}", var);
}

fn ruby_version() -> Option<Version> {
    Some(env::var_os("ALOXIDE_RUBY_VERSION")?
        .to_str()
        .expect("'ALOXIDE_RUBY_VERSION' is not UTF-8")
        .parse()
        .expect("Could not parse 'ALOXIDE_RUBY_VERSION'"))
}

fn main() {
    rerun_if_env_changed("ALOXIDE_USE_RVM");
    rerun_if_env_changed("ALOXIDE_USE_RBENV");
    rerun_if_env_changed("ALOXIDE_RUBY_VERSION");
    rerun_if_env_changed("ALOXIDE_STATIC_RUBY");

    let static_lib = has_env("ALOXIDE_STATIC_RUBY");

    let ruby = match (Driver::get(), ruby_version()) {
        (Some(driver), version) => {
            let version = version.unwrap_or(Version::new(2, 6, 2));
            let ruby = driver.ruby(&version);
            assert_eq!(*ruby.version(), version);
            ruby
        },
        (None, Some(version)) => {
            build_ruby(&version, static_lib)
        },
        (None, None) => {
            Ruby::current().expect("Could not get system Ruby")
        },
    };

    let version = ruby.version();
    println!("Building for Ruby {}", version);

    println!("{}", config(&ruby));

    ruby.link(static_lib).unwrap();
}
