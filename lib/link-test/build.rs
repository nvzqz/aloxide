extern crate aloxide;

use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use aloxide::{Ruby, Version};

fn main() {
    let target = env::var("TARGET").unwrap();

    let version = match env::var("ALOXIDE_RUBY_VERSION") {
        Ok(ref version) if !version.is_empty() => {
            version.parse::<Version>().unwrap()
        },
        _ => Version::new(2, 6, 2),
    };

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let aloxide = manifest_dir
        .parent().unwrap()
        .parent().unwrap()
        .join("target")
        .join("aloxide");
    assert!(aloxide.parent().unwrap().exists());

    let target_dir = aloxide.join(&target);
    let out_dir = target_dir.join(&format!("ruby-{}-out", version));

    println!("Downloading Ruby {} into '{}'", version, target_dir.display());

    let cache = env::var_os("ALOXIDE_RUBY_CACHE");
    let mut downloader = Ruby::src_downloader(version, &target_dir);
    if let Some(cache) = &cache {
        downloader = downloader.cache_dir(cache);
    }
    let src_dir = downloader.cache().download().unwrap();

    println!("Compiling sources in '{}' to '{}'", src_dir.display(), out_dir.display());

    let ruby = Ruby::builder(&src_dir, &out_dir, target)
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
        .unwrap();

    println!("{}", ruby.run("require 'pp'; pp RbConfig::CONFIG").unwrap());

    ruby.link(true).unwrap();
}
