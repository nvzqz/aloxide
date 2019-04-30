extern crate aloxide;

use std::path::PathBuf;
use std::process::Stdio;
use aloxide::{Ruby, Version};

fn main() {
    let target = std::env::var("TARGET").unwrap();

    let version = if let Ok(version) = std::env::var("ALOXIDE_RUBY_VERSION") {
        version.parse::<Version>().unwrap()
    } else {
        Version::new(2, 6, 2)
    };

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let aloxide = manifest_dir.join("..")
        .join("..")
        .join("target")
        .join("aloxide");
    assert!(aloxide.parent().unwrap().exists());

    let target_dir = aloxide.join(&target);
    let src_dir = Ruby::src_downloader(version, &target_dir)
        .cache()
        .download()
        .unwrap();
    let out_dir = target_dir.join(&format!("ruby-{}-out", version));

    let ruby = Ruby::builder(&src_dir, &out_dir, target)
        .autoconf_stdout(Stdio::inherit())
        .autoconf_stderr(Stdio::inherit())
        .configure_stdout(Stdio::inherit())
        .configure_stderr(Stdio::inherit())
        .make_stdout(Stdio::inherit())
        .make_stderr(Stdio::inherit())
        .build()
        .unwrap();

    println!("{}", ruby.run("require 'pp'; pp RbConfig::CONFIG").unwrap());

    ruby.link(true).unwrap();
}
