use std::collections::HashSet;
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

// e.g. "-llibruby"
fn lib_name(lib_flag: &str) -> &str {
    &lib_flag[2..]
}

// e.g. "user32.lib"
fn lib_name_msvc(lib_flag: &str) -> &str {
    &lib_flag[..(lib_flag.len() - 4)]
}

pub(crate) fn link(ruby: &Ruby, static_lib: bool) -> Result<(), RubyLinkError> {
    println!("cargo:rustc-link-search=native={}", ruby.lib_path.display());

    let target = ruby.get_config("target")?;
    let target_msvc = target.contains("msvc") || target.contains("mswin");
    let lib_name = if target_msvc { lib_name_msvc } else { lib_name };

    let key = if static_lib {
        "LIBRUBYARG_STATIC"
    } else {
        "LIBRUBYARG_SHARED"
    };
    let args = ruby.get_config(key)?;

    if args.trim().is_empty() {
        return Err(RubyLinkError::MissingLibs { static_lib });
    }

    let so_libs = ruby.so_libs()?;
    let aux_libs = ruby.aux_libs(static_lib)?;

    // TODO: `MAINLIBS` can be `nil` on Windows, and so `aux_libs()` should make
    // use of `Option<String>` instead
    let aux_libs = if aux_libs != "nil" {
        aux_libs.as_str()
    } else {
        ""
    };

    let mut dy_libs = HashSet::new();
    dy_libs.extend(aux_libs.split_ascii_whitespace().map(lib_name));
    dy_libs.extend(so_libs.split_ascii_whitespace().map(lib_name));

    let ruby_lib = ruby.lib_name(static_lib)?;
    if static_lib {
        link_static(&ruby_lib);
    } else {
        link_dynamic(&ruby_lib);
    }

    let seen_lib = |lib: &str| {
        lib == ruby_lib || dy_libs.contains(lib)
    };

    for lib in &dy_libs {
        link_dynamic(lib);
    }

    // TODO: Figure out whether `args` should be evaluated for MSVC
    if target_msvc {
        return Ok(());
    }

    // Need to call `next()` in "-framework" case
    let mut args_iter = args.split_ascii_whitespace();

    while let Some(arg) = args_iter.next() {
        if arg.len() < 2 {
            panic!("Unknown arg {:?} in {:?}", arg, args);
        }
        let (opt, val) = arg.split_at(2);
        match opt {
            "-l" => if !seen_lib(val) {
                link_dynamic(val);
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
                let framework = match args_iter.next() {
                    Some(arg) => arg,
                    None => return Err(MissingFramework(args)),
                };
                link_framework(framework);
            } else {
                panic!("Unknown arg {:?} in {:?}", arg, args);
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
