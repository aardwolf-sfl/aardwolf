pub mod api;
#[macro_use]
pub mod arena;
pub mod config;
pub mod data;
mod driver;
mod graph_ext;
mod logger;
pub mod plugins;
pub mod structures;
mod ui;

use clap::{App, Arg};

use driver::{Driver, DriverArgs};
use ui::UiName;

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
            Arg::with_name("ui")
                .long("ui")
                .takes_value(true)
                .possible_values(&["cli", "json"])
                .help(""),
        )
        .get_matches();

    let args = DriverArgs::new()
        .with_config_path(matches.value_of("config"))
        .with_ui(
            matches
                .value_of("ui")
                .map(|ui| match ui {
                    "cli" => UiName::Cli,
                    "json" => UiName::Json,
                    _ => unreachable!(),
                })
                .unwrap_or_default(),
        );

    Driver::run(&args);
}
