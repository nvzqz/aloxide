extern crate aloxide;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use aloxide::{Ruby, RubySrc, Version};

// The driver that manages the Ruby installation
enum Driver {
    // https://github.com/rvm/rvm
    Rvm,
    // https://www.github.com/rbenv/rbenv
    Rbenv,
    // This project - https://github.com/nvzqz/aloxide
    Aloxide,
}

impl Driver {
    fn get() -> Driver {
        if env::var_os("ALOXIDE_USE_RVM").is_some() {
            Driver::Rvm
        } else if env::var_os("ALOXIDE_USE_RBENV").is_some() {
            Driver::Rbenv
        } else {
            Driver::Aloxide
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
            Driver::Aloxide => return build_ruby(version),
        }
    }
}

fn build_ruby(version: &Version) -> Ruby {
    let shared_lib = cfg!(feature = "shared");
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
            .shared_lib(shared_lib)
            .disable_install_doc()
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
        .make()
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
        .build()
        .expect(&format!("Failed to build Ruby {}", version))
}

fn config(ruby: &Ruby) -> String {
    ruby.run("require 'pp'; pp RbConfig::CONFIG").expect("Failed to get config")
}

fn main() {
    let version = match env::var("ALOXIDE_RUBY_VERSION") {
        Ok(ref version) if !version.is_empty() => {
            version.parse::<Version>().unwrap()
        },
        _ => Version::new(2, 6, 2),
    };

    let ruby = Driver::get().ruby(&version);

    let config_path = ruby
        .out_dir()
        .parent().unwrap()
        .join(format!("ruby-{}-config.txt", version));

    let mut config_file = File::create(&config_path)
        .expect(&format!("Failed to create {:?}", config_path));

    let config = config(&ruby);

    println!("{}", config);
    write!(config_file, "{}", config)
        .expect(&format!("Failed to write to {:?}", config_path));

    let static_lib = !cfg!(feature = "shared");

    let aux_libs = ruby.aux_libs(static_lib).unwrap();
    println!("Linking to aux libs: {}", aux_libs);

    ruby.link(static_lib).unwrap();
}
