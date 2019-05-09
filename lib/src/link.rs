use crate::{Ruby, RubyExecError};
use RubyLinkError::*;

fn link_static(lib: &str) {
    println!("cargo:rustc-link-lib=static={}", lib);
}

fn link_dynamic(lib: &str) {
    println!("cargo:rustc-link-lib=dylib={}", lib);
}

fn link_framework(lib: &str) {
    println!("cargo:rustc-link-lib=framework={}", lib);
}

pub(crate) fn link(ruby: &Ruby, static_lib: bool) -> Result<(), RubyLinkError> {
    println!("cargo:rustc-link-search=native={}", ruby.lib_path.display());

    let key = if static_lib {
        "LIBRUBYARG_STATIC"
    } else {
        "LIBRUBYARG_SHARED"
    };

    let libs = ruby.aux_libs(static_lib)?;
    let args = ruby.get_config(key)?;

    if args.trim().is_empty() {
        return Err(RubyLinkError::MissingLibs { static_lib });
    }

    let link_lib = |lib| {
        if libs.contains(lib) { return; }
        if static_lib {
            link_static(lib);
        } else {
            link_dynamic(lib);
        }
    };

    let target = ruby.get_config("target")?;
    let is_msvc = target.contains("msvc") || target.contains("mswin");

    if is_msvc {
        fn link_libs<'a>(libs: &'a str, link: impl Fn(&'a str) -> ()) {
            for lib in libs.split_ascii_whitespace() {
                if lib.ends_with(".lib") {
                    let name_len = lib.len() - 4;
                    link(&lib[..name_len]);
                } else {
                    unimplemented!("{:?}", libs);
                }
            }
        }
        link_libs(&libs, link_dynamic);
        link_libs(&args, link_lib);
        return Ok(());
    }

    for lib in libs.split_ascii_whitespace() {
        link_dynamic(&lib[2..]);
    }

    let mut iter = args.split_ascii_whitespace();
    while let Some(arg) = iter.next() {
        if arg.len() < 2 {
            unimplemented!("{:?}", args);
        }
        let (opt, val) = arg.split_at(2);
        match opt {
            "-l" => {
                link_lib(val);
            },
            "-L" => {
                println!("cargo:rustc-link-search=native={}", val);
            },
            "-F" => {
                println!("cargo:rustc-link-search=framework={}", val);
            },
            "-W" => {
                continue;
            },
            _ => if arg == "-framework" {
                let framework = match iter.next() {
                    Some(arg) => arg,
                    None => return Err(MissingFramework(args)),
                };
                link_framework(framework);
            } else {
                unimplemented!("{:?}", args);
            }
        }
    }

    Ok(())
}

/// The error returned when linking to the Ruby library and its dependencies
/// fails.
#[derive(Debug)]
pub enum RubyLinkError {
    /// Failed to execute the `ruby` binary.
    Exec(RubyExecError),
    /// A `-framework` flag was found with no argument.
    MissingFramework(String),
    /// Libraries for the type of linking could not be found.
    MissingLibs {
        /// Whether linking to Ruby statically.
        static_lib: bool
    },
}

impl From<RubyExecError> for RubyLinkError {
    #[inline]
    fn from(error: RubyExecError) -> Self {
        RubyLinkError::Exec(error)
    }
}
