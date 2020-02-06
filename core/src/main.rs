#![allow(warnings)]

pub mod api;
pub mod config;
mod driver;
pub mod plugins;
pub mod raw;
pub mod structures;

use clap::{App, Arg};

use driver::{Driver, DriverArgs};

fn main() {
    let matches = App::new("aardwolf")
        .version("0.1")
        .author("Petr Nevyhoštěný")
        .about("A Modular and Extensible Tool for Software Fault Localization")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                // Sets path to the config file. By default it searches for .aardwolf.yml in the current parent directories.
                .help("")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("runtime")
                .short("r")
                .long("runtime")
                .value_name("FILE")
                .help("")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("frontend")
                .long("frontend")
                .value_name("EXECUTABLE")
                .help("")
                .takes_value(true),
        )
        .get_matches();

    let args = DriverArgs::new(
        matches
            .value_of("runtime")
            // Default path to runtime library expects it to be in a system path.
            .unwrap_or("libaardwolf_runtime.a"),
    )
    .with_config_path(matches.value_of("config"))
    .with_frontend_path(matches.value_of("frontend"));

    Driver::run(&args);
}
