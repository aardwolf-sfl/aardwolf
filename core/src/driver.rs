//! Aardwolf analysis driver.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use crate::api::Api;
use crate::config::{Config, LoadConfigError};
use crate::data::{ParseError, RawData};
use crate::logger::Logger;
use crate::plugins::{
    collect_bb::CollectBb, invariants::Invariants, irrelevant::Irrelevant, prob_graph::ProbGraph,
    sbfl::Sbfl, AardwolfPlugin, Metadata, NormalizedResults, PluginError, PluginInitError,
    Preprocessing, Results,
};
use crate::ui::{CliUi, JsonUi, Ui, UiName};

pub const TRACE_FILE: &'static str = "aard.trace";
pub const RESULT_FILE: &'static str = "aard.result";
pub const LOG_FILE: &'static str = "aard.log";

pub const DEFAULT_CONFIG_FILE: &'static str = ".aardwolf.yml";
pub const DEFAULT_SHELL: &'static str = "bash";

/// Collection of file paths used by the driver.;
pub struct DriverPaths {
    /// Output directory for Aardwolf-related data.
    pub output_dir: PathBuf,
    /// Working directory where `.aardwolf.yml` is located.
    pub work_dir: PathBuf,
    /// Directory where Aardwolf artifacts (binaries and libraries) are located.
    pub aardwolf_dir: PathBuf,
    /// The absolute path to the trace file.
    pub trace_file: PathBuf,
    /// The absolute path to the test results file.
    pub result_file: PathBuf,
}

impl DriverPaths {
    /// Sets all driver paths using given config and its location.
    pub fn new<P: AsRef<Path>>(config: &Config, config_path: P) -> io::Result<Self> {
        let output_dir = config_path.as_ref().join(&config.output_dir);
        let current_exe = env::current_exe()?;

        Ok(DriverPaths {
            trace_file: output_dir.join(TRACE_FILE),
            result_file: output_dir.join(RESULT_FILE),
            output_dir,
            work_dir: config_path.as_ref().to_path_buf(),
            aardwolf_dir: current_exe.parent().unwrap().to_path_buf(),
        })
    }
}

/// Command line arguments for the driver.
pub struct DriverArgs<P: AsRef<Path>> {
    config_path: Option<P>,
    ui: UiName,
    reuse: bool,
    ignore_corrupted: bool,
}

impl<P: AsRef<Path>> DriverArgs<P> {
    /// Creates default arguments.
    pub fn new() -> Self {
        DriverArgs {
            config_path: None,
            ui: UiName::default(),
            reuse: false,
            ignore_corrupted: false,
        }
    }

    /// Sets the configuration filepath.
    pub fn with_config_path(self, config_path: Option<P>) -> Self {
        Self {
            config_path,
            ..self
        }
    }

    /// Sets the UI name.
    pub fn with_ui(self, ui: UiName) -> Self {
        Self { ui, ..self }
    }

    /// Indicates to the driver whether it should reuse already generated data.
    pub fn with_reuse(self, reuse: bool) -> Self {
        Self { reuse, ..self }
    }

    pub fn with_ignore_corrupted(self, ignore_corrupted: bool) -> Self {
        Self {
            ignore_corrupted,
            ..self
        }
    }
}

// Process localization plugins as they are defined in the config.
// Implement ordering by its index.
#[derive(Eq)]
struct LocalizationId<'a>(&'a str, usize);

impl<'a> LocalizationId<'a> {
    pub fn new(name: &'a str, index: usize) -> Self {
        LocalizationId(name, index)
    }
}

impl<'a> Ord for LocalizationId<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.1.cmp(&other.1)
    }
}

impl<'a> PartialOrd for LocalizationId<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> PartialEq for LocalizationId<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

/// The main driver for the Aardwolf analysis.
pub struct Driver;

impl Driver {
    /// Runs the analysis.
    pub fn run<P: AsRef<Path>>(args: &DriverArgs<P>) {
        let mut ui: Box<dyn Ui> = match args.ui {
            UiName::Cli => Box::new(CliUi::new().expect("Standard output is inaccessible.")),
            UiName::Json => Box::new(JsonUi::new()),
        };

        let (config, config_path) = ui.unwrap(Self::load_config(args.config_path.as_ref()));
        let driver_paths = ui.unwrap(DriverPaths::new(&config, &config_path));

        if !args.reuse {
            ui.unwrap(Self::prepare(&driver_paths));
        }

        let mut logger = Logger::new(driver_paths.output_dir.join(LOG_FILE));
        logger.info("config file loaded");

        if !args.reuse {
            let script_handle = logger.perf("run script");
            ui.unwrap(Self::run_script(&config, &driver_paths));
            script_handle.stop();
        }

        let data_handle = logger.perf("load data");
        let data = ui.unwrap(Self::load_data(&driver_paths, args.ignore_corrupted));
        data_handle.stop();

        let api = ui.unwrap(Api::new(data));

        let init_handle = logger.perf("init plugins");
        let plugins = ui.unwrap(Self::init_plugins(&config, &api));
        init_handle.stop();

        let (results, metadata) = ui.unwrap(Self::run_loc(&config, &api, &plugins, &mut logger));

        let display_handle = logger.perf("display results");
        Self::display_results(ui, &config, &api, results, metadata);
        display_handle.stop();
    }

    fn prepare(driver_paths: &DriverPaths) -> io::Result<()> {
        fs::create_dir_all(&driver_paths.output_dir)?;
        // Remove Aardwolf-related files.
        Self::clean_dir(&driver_paths.output_dir)
    }

    fn clean_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
        for file in fs::read_dir(&path)? {
            let path = file?.path();

            if path.is_dir() {
                Self::clean_dir(&path)?;
            } else if path.is_file() {
                let filename = path.file_name().unwrap().to_string_lossy();
                if filename.ends_with(".aard") || filename.starts_with("aard.") {
                    fs::remove_file(&path)?;
                }
            }
        }

        Ok(())
    }

    // TODO: Make return type so it can also show eventual script stderr/stdout.
    fn run_script(config: &Config, driver_paths: &DriverPaths) -> Result<(), RunScriptError> {
        let path = env::temp_dir().join(format!("aardwolf.{}", process::id()));
        let mut file = File::create(&path).map_err(RunScriptError::Initialization)?;
        file.write_all(config.script.join("\n").as_bytes())
            .map_err(RunScriptError::Initialization)?;

        let exit_status = Command::new(DEFAULT_SHELL)
            .arg("-o")
            .arg("errexit")
            .arg(path)
            .env("OUTPUT_DIR", &driver_paths.output_dir)
            .env("AARDWOLF_DATA_DEST", &driver_paths.output_dir)
            .env("WORK_DIR", &driver_paths.work_dir)
            .env("AARDWOLF_DIR", &driver_paths.aardwolf_dir)
            .env("TRACE_FILE", &driver_paths.trace_file)
            .env("RESULT_FILE", &driver_paths.result_file)
            .status()
            .map_err(RunScriptError::Execution)?;

        if exit_status.success() {
            Ok(())
        } else {
            Err(RunScriptError::ExitStatus(exit_status.code()))
        }
    }

    fn load_data(
        driver_paths: &DriverPaths,
        ignore_corrupted: bool,
    ) -> Result<RawData, LoadDataError> {
        let mut static_files = Self::find_static_files(driver_paths);
        let mut dynamic_file =
            BufReader::new(File::open(&driver_paths.trace_file).map_err(LoadDataError::Io)?);
        let mut test_file =
            BufReader::new(File::open(&driver_paths.result_file).map_err(LoadDataError::Io)?);

        RawData::parse(
            static_files.iter_mut(),
            &mut dynamic_file,
            &mut test_file,
            ignore_corrupted,
        )
        .map_err(LoadDataError::Parse)
    }

    fn find_static_files(driver_paths: &DriverPaths) -> Vec<BufReader<File>> {
        let mut files = Vec::new();

        let mut dirs = vec![driver_paths.output_dir.clone()];

        while let Some(dir) = dirs.pop() {
            if let Ok(entries) = dir.read_dir() {
                for entry in entries.filter_map(|entry| entry.ok()) {
                    let entry_path = entry.path();

                    if entry_path.is_file() {
                        if let Some("aard") =
                            entry_path.extension().map(|ext| ext.to_str().unwrap())
                        {
                            files.push(BufReader::new(File::open(entry_path).unwrap()));
                        }
                    } else if entry_path.is_dir() {
                        dirs.push(entry_path);
                    }
                }
            }
        }

        files
    }

    fn load_config<P: AsRef<Path>>(
        config_path: Option<P>,
    ) -> Result<(Config, PathBuf), LoadConfigError> {
        match config_path {
            Some(config_path) => Config::load_from_file(&config_path)
                .map(|config| (config, config_path.as_ref().parent().unwrap().to_path_buf())),
            None => {
                let current_dir = env::current_dir().map_err(|err| LoadConfigError::Io(err))?;

                // Perform directory read to check if we have permissions for current directory.
                // If we encounter io error in the directories higher in the tree directory tree,
                // we assume that we got to places where we are forbidden to be, and return
                // "not found" error instead of "io error".
                current_dir
                    .read_dir()
                    .map_err(|err| LoadConfigError::Io(err))?;
                let mut current_dir = Some(current_dir.as_path());
                // Find `.aardwolf.yml` file in the directory tree.
                while let Some(dir) = current_dir {
                    for entry in dir.read_dir().map_err(|_| LoadConfigError::NotFound)? {
                        if let Ok(entry) = entry {
                            let entry_path = entry.path();
                            if entry_path.is_file() && entry_path.ends_with(DEFAULT_CONFIG_FILE) {
                                return Config::load_from_file(&entry_path).map(|config| {
                                    (config, entry_path.parent().unwrap().to_path_buf())
                                });
                            }
                        }
                    }
                    current_dir = dir.parent();
                }
                Err(LoadConfigError::NotFound)
            }
        }
    }

    fn init_plugins<'a>(
        config: &'a Config,
        api: &'a Api,
    ) -> Result<Vec<(&'a str, Box<dyn AardwolfPlugin>)>, PluginInitError> {
        let mut plugins = Vec::with_capacity(config.plugins.len());

        for plugin in config.plugins.iter() {
            let name = plugin.id();

            let plugin: Box<dyn AardwolfPlugin> = match plugin.id.as_str() {
                "sbfl" => Box::new(Sbfl::init(&api, &plugin.opts)?),
                "prob-graph" => Box::new(ProbGraph::init(&api, &plugin.opts)?),
                "invariants" => Box::new(Invariants::init(&api, &plugin.opts)?),
                "collect-bb" => Box::new(CollectBb::init(&api, &plugin.opts)?),
                "irrelevant" => Box::new(Irrelevant::init(&api, &plugin.opts)?),
                _ => return Err(format!("unknown plugin \"{}\"", name)),
            };

            plugins.push((name, plugin));
        }

        Ok(plugins)
    }

    fn run_loc<'a>(
        config: &'a Config,
        api: &'a Api,
        plugins: &'a Vec<(&'a str, Box<dyn AardwolfPlugin>)>,
        logger: &mut Logger,
    ) -> Result<(BTreeMap<LocalizationId<'a>, NormalizedResults>, Metadata), PluginError> {
        let mut preprocessing = Preprocessing::new(&api);

        for (name, plugin) in plugins {
            let handle = logger.perf(format!("{} (pre)", name));
            plugin.run_pre(&api, &mut preprocessing)?;
            handle.stop();
        }

        let mut all_results = BTreeMap::new();

        for (name, plugin) in plugins {
            let id = LocalizationId::new(name, all_results.len());
            let mut results = Results::new(Self::n_results(config, &id));

            let handle = logger.perf(format!("{} (loc)", name));
            plugin.run_loc(&api, &mut results, &preprocessing)?;
            handle.stop();

            if results.any() {
                all_results.insert(id, results.normalize());
            }
        }

        let all_results_by_name = all_results
            .iter()
            .map(|(id, results)| (id.0, results))
            .collect::<HashMap<_, _>>();

        let mut post_results = BTreeMap::new();
        let mut metadata = Metadata::new();

        for (name, plugin) in plugins {
            let id = LocalizationId::new(name, all_results.len());
            let mut results = Results::new(Self::n_results(config, &id));

            let handle = logger.perf(format!("{} (post)", name));
            plugin.run_post(&api, &all_results_by_name, &mut results, &mut metadata)?;
            handle.stop();

            if results.any() {
                post_results.insert(id, results.normalize());
            }
        }

        for (id, results) in post_results {
            all_results.insert(id, results);
        }

        Ok((all_results, metadata))
    }

    fn display_results<'a>(
        mut ui: Box<dyn Ui>,
        config: &'a Config,
        api: &'a Api,
        results: BTreeMap<LocalizationId<'a>, NormalizedResults>,
        metadata: Metadata,
    ) {
        ui.prolog(api);

        for (id, results) in results.into_iter() {
            if Self::should_display(config, &id) {
                ui.plugin(id.0, api);

                for item in results {
                    ui.result(&item, api);
                }
            }
        }

        ui.metadata(&metadata, api);

        ui.epilog(api);
    }

    fn n_results<'a>(config: &'a Config, id: &LocalizationId<'a>) -> usize {
        for plugin in config.plugins.iter() {
            if plugin.id() == id.0 {
                if let Some(n_results) = plugin
                    .opts
                    .get("n_results")
                    .and_then(|n_results| n_results.as_i64())
                {
                    return n_results as usize;
                }
            }
        }

        config.n_results
    }

    fn should_display<'a>(config: &'a Config, id: &LocalizationId<'a>) -> bool {
        for plugin in config.plugins.iter() {
            if plugin.id() == id.0 {
                if let Some(false) = plugin
                    .opts
                    .get("display")
                    .and_then(|display| display.as_bool())
                {
                    return false;
                }
            }
        }

        true
    }
}

enum RunScriptError {
    Initialization(io::Error),
    Execution(io::Error),
    ExitStatus(Option<i32>),
}

impl fmt::Display for RunScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunScriptError::Initialization(error) => {
                write!(f, "script could not be initialized: {}", error)
            }
            RunScriptError::Execution(error) => {
                write!(f, "script could not be executed: {}", error)
            }
            RunScriptError::ExitStatus(Some(exit_code)) => write!(
                f,
                "script finished with a non-zero exit code: {}",
                exit_code
            ),
            RunScriptError::ExitStatus(None) => write!(f, "script was terminated by a signal"),
        }
    }
}

enum LoadDataError {
    Io(io::Error),
    Parse(ParseError),
}

impl fmt::Display for LoadDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadDataError::Io(error) => write!(f, "{}", error),
            LoadDataError::Parse(error) => write!(f, "parsing data failed: {}", error),
        }
    }
}

trait ErrorHandler {
    fn unwrap<T, E>(&mut self, result: Result<T, E>) -> T
    where
        E: std::fmt::Display;
}

impl ErrorHandler for Box<dyn Ui> {
    fn unwrap<T, E>(&mut self, result: Result<T, E>) -> T
    where
        E: std::fmt::Display,
    {
        match result {
            Ok(value) => value,
            Err(error) => {
                self.error(&format!("{}", error));
                std::process::exit(1);
            }
        }
    }
}
