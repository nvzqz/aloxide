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

    let dylibs = ruby.main_libs()?;
    let args = ruby.get_config(key)?;

    let link_lib = |lib| {
        if !static_lib || dylibs.contains(lib) {
            link_dynamic(lib);
        } else {
            link_static(lib);
        }
    };

    let mut iter = args.split_ascii_whitespace();

    let target = ruby.get_config("target")?;
    let is_msvc = target.contains("msvc") || target.contains("mswin");

    if is_msvc {
        for lib in iter {
            if lib.ends_with(".lib") {
                let name_len = lib.len() - 4;
                link_lib(&lib[..name_len]);
            } else {
                unimplemented!("{:?}", args);
            }
        }
        return Ok(());
    }

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
}

impl From<RubyExecError> for RubyLinkError {
    #[inline]
    fn from(error: RubyExecError) -> Self {
        RubyLinkError::Exec(error)
    }
}
