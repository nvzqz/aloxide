extern crate aloxide;

use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use aloxide::{RubySrc, Version};

fn main() {
    let target = env::var("TARGET").unwrap();

    let version = match env::var("ALOXIDE_RUBY_VERSION") {
        Ok(ref version) if !version.is_empty() => {
            version.parse::<Version>().unwrap()
        },
        _ => Version::new(2, 6, 2),
    };

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
    let mut downloader = RubySrc::downloader(&version, &target_dir);
    if let Some(cache) = &cache {
        downloader = downloader.cache_dir(cache);
    }

    let ruby = downloader
        .cache()
        .download()
        .expect(&format!("Failed to download Ruby {}", version))
        .builder(out_dir, target)
        .autoconf()
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
        .configure()
            .inherit_cc()
            .disable_install_doc()
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
        .make()
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
        .build()
        .expect(&format!("Failed to build Ruby {}", version));

    println!("{}", ruby.run("require 'pp'; pp RbConfig::CONFIG").unwrap());

    ruby.link(true).unwrap();
}
