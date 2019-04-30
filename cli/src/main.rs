extern crate aloxide;
extern crate clap;

use aloxide::{version::{Version, VersionParseError}};
use clap::{Arg, ArgMatches, ArgSettings, App, AppSettings, SubCommand};

macro_rules! error {
    ($($t:tt)+) => { { eprintln!($($t)+); std::process::exit(1) } }
}

fn main() {
    let app = App::new("aloxide")
        .about("Build and install different Ruby versions")
        .author("Nikolai Vazquez")
        .settings(&[
            AppSettings::SubcommandRequiredElseHelp,
        ])
        .set_term_width(80)
        .args(&[
            Arg::with_name("v")
                .long("verbose")
                .short("v")
                .help("Output extra information")
                .set(ArgSettings::Global)
        ])
        .subcommands(vec![
            SubCommand::with_name("build")
                .about("Build a specific Ruby version")
                .args(&[
                    Arg::with_name("version")
                        .takes_value(true)
                        .required(true),
                    Arg::with_name("output")
                        .long("out")
                        .short("o")
                        .help("Specifies where Ruby should be built")
                        .takes_value(true),
                ]),
        ]);
    let matches = app.get_matches();

    match matches.subcommand() {
        ("build", Some(matches)) => build_ruby(matches),
        _ => unreachable!(),
    }
}

fn get_version(matches: &ArgMatches) -> Option<Result<Version, VersionParseError>> {
    let version = matches.value_of("version")?;
    Some(Version::parser().require_minor().parse(version))
}

fn build_ruby(matches: &ArgMatches) {
    let version = match get_version(matches) {
        Some(Ok(value)) => value,
        Some(Err(_)) => {
            error!("Version is required to be in the format 'x.y' or 'x.y.z'");
        }
        None => {
            error!("Version not provided");
        },
    };

    unimplemented!("TODO: Implement downloading Ruby {}", version);
}
