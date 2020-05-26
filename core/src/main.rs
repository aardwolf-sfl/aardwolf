pub mod api;
#[macro_use]
pub mod arena;
pub mod config;
pub mod data;
mod driver;
mod graph_ext;
mod logger;
pub mod plugins;
pub mod queries;
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
                .help("Sets the path to the configuration file. By default, it searches the current directory and its parents until `.aardwolf.yml` is found.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("ui")
                .long("ui")
                .takes_value(true)
                .possible_values(&["cli", "json"])
                .help("Sets the UI which will be used for results presentation."),
        )
        .arg(Arg::with_name("reuse").long("reuse").help("If set to true, Aardwolf will not run the script and will reuse already generated data which are expected to exist."))
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
        )
        .with_reuse(matches.is_present("reuse"));

    Driver::run(&args);
}
