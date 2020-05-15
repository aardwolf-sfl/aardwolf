use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::{self, File};
use std::io::{self, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use crate::api::Api;
use crate::config::{Config, LoadConfigError};
use crate::data::RawData;
use crate::logger::Logger;
use crate::plugins::{
    collect_bb::CollectBb, invariants::Invariants, irrelevant::Irrelevant, prob_graph::ProbGraph,
    sbfl::Sbfl, AardwolfPlugin, IrrelevantItems, NormalizedResults, Results,
};
use crate::ui::{CliUi, JsonUi, Ui, UiName};

pub const TRACE_FILE: &'static str = "aard.trace";
pub const RESULT_FILE: &'static str = "aard.result";
pub const LOG_FILE: &'static str = "aard.log";

pub const DEFAULT_CONFIG_FILE: &'static str = ".aardwolf.yml";
pub const DEFAULT_SHELL: &'static str = "bash";

pub struct DriverPaths {
    pub output_dir: PathBuf,
    pub work_dir: PathBuf,
    pub aardwolf_dir: PathBuf,
    pub trace_file: PathBuf,
    pub result_file: PathBuf,
}

impl DriverPaths {
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

pub struct DriverArgs<P: AsRef<Path>> {
    config_path: Option<P>,
    ui: UiName,
    reuse: bool,
}

impl<P: AsRef<Path>> DriverArgs<P> {
    pub fn new() -> Self {
        DriverArgs {
            config_path: None,
            ui: UiName::default(),
            reuse: false,
        }
    }

    pub fn with_config_path(self, config_path: Option<P>) -> Self {
        Self {
            config_path,
            ..self
        }
    }

    pub fn with_ui(self, ui: UiName) -> Self {
        Self { ui, ..self }
    }

    pub fn with_reuse(self, reuse: bool) -> Self {
        Self { reuse, ..self }
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

pub struct Driver;

impl Driver {
    pub fn run<P: AsRef<Path>>(args: &DriverArgs<P>) {
        let (config, config_path) = Self::load_config(args.config_path.as_ref()).unwrap();
        let driver_paths = DriverPaths::new(&config, &config_path).unwrap();

        fs::create_dir_all(&driver_paths.output_dir).unwrap();

        let mut logger = Logger::new(driver_paths.output_dir.join(LOG_FILE));
        logger.info("config file loaded");

        if !args.reuse {
            let script_handle = logger.perf("run script");
            Self::run_script(&config, &driver_paths).unwrap();
            script_handle.stop();
        }

        let data_handle = logger.perf("load data");
        let data = Self::load_data(&driver_paths);
        data_handle.stop();

        let api = Api::new(data).unwrap();

        let init_handle = logger.perf("init plugins");
        let plugins = Self::init_plugins(&config, &api);
        init_handle.stop();

        let results = Self::run_loc(&config, &api, &plugins, &mut logger);

        let display_handle = logger.perf("display results");
        Self::display_results(args.ui, &config, &api, results);
        display_handle.stop();
    }

    // TODO: Make return type so it can also show eventual script stderr/stdout.
    fn run_script(config: &Config, driver_paths: &DriverPaths) -> Result<(), ()> {
        let path = env::temp_dir().join(format!("aardwolf.{}", process::id()));
        let mut file = File::create(&path).unwrap();
        file.write_all(config.script.join("\n").as_bytes()).unwrap();

        Command::new(DEFAULT_SHELL)
            .arg(path)
            .env("OUTPUT_DIR", &driver_paths.output_dir)
            .env("AARDWOLF_DATA_DEST", &driver_paths.output_dir)
            .env("WORK_DIR", &driver_paths.work_dir)
            .env("AARDWOLF_DIR", &driver_paths.aardwolf_dir)
            .env("TRACE_FILE", &driver_paths.trace_file)
            .env("RESULT_FILE", &driver_paths.result_file)
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        Ok(())
    }

    fn load_data(driver_paths: &DriverPaths) -> RawData {
        let mut static_files = Self::find_static_files(driver_paths);
        let mut dynamic_file = BufReader::new(File::open(&driver_paths.trace_file).unwrap());
        let mut test_file = BufReader::new(File::open(&driver_paths.result_file).unwrap());

        RawData::parse(static_files.iter_mut(), &mut dynamic_file, &mut test_file).unwrap()
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
    ) -> Vec<(&'a str, Box<dyn AardwolfPlugin>)> {
        config
            .plugins
            .iter()
            .map(|plugin| {
                let name = plugin.id();

                let plugin: Box<dyn AardwolfPlugin> = match plugin.id.as_str() {
                    "sbfl" => Box::new(Sbfl::init(&api, &plugin.opts).unwrap()),
                    "prob-graph" => Box::new(ProbGraph::init(&api, &plugin.opts).unwrap()),
                    "invariants" => Box::new(Invariants::init(&api, &plugin.opts).unwrap()),
                    "collect-bb" => Box::new(CollectBb::init(&api, &plugin.opts).unwrap()),
                    "irrelevant" => Box::new(Irrelevant::init(&api, &plugin.opts).unwrap()),
                    _ => panic!("Unknown plugin"),
                };

                (name, plugin)
            })
            .collect()
    }

    fn run_loc<'a>(
        config: &'a Config,
        api: &'a Api,
        plugins: &'a Vec<(&'a str, Box<dyn AardwolfPlugin>)>,
        logger: &mut Logger,
    ) -> BTreeMap<LocalizationId<'a>, NormalizedResults> {
        let mut preprocessing = IrrelevantItems::new(&api);

        for (name, plugin) in plugins {
            let handle = logger.perf(format!("{} (pre)", name));
            plugin.run_pre(&api, &mut preprocessing).unwrap();
            handle.stop();
        }

        let mut all_results = BTreeMap::new();

        for (name, plugin) in plugins {
            let id = LocalizationId::new(name, all_results.len());
            let mut results = Results::new(Self::n_results(config, &id));

            let handle = logger.perf(format!("{} (loc)", name));
            plugin.run_loc(&api, &mut results, &preprocessing).unwrap();
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

        for (name, plugin) in plugins {
            let id = LocalizationId::new(name, all_results.len());
            let mut results = Results::new(Self::n_results(config, &id));

            let handle = logger.perf(format!("{} (post)", name));
            plugin
                .run_post(&api, &all_results_by_name, &mut results)
                .unwrap();
            handle.stop();

            if results.any() {
                post_results.insert(id, results.normalize());
            }
        }

        for (id, results) in post_results {
            all_results.insert(id, results);
        }

        all_results
    }

    fn display_results<'a>(
        ui: UiName,
        config: &'a Config,
        api: &'a Api,
        results: BTreeMap<LocalizationId<'a>, NormalizedResults>,
    ) {
        let mut ui: Box<dyn Ui> = match ui {
            UiName::Cli => Box::new(CliUi::new(api).unwrap()),
            UiName::Json => Box::new(JsonUi::new(api)),
        };

        ui.prolog();

        for (id, results) in results.into_iter() {
            if Self::should_display(config, &id) {
                ui.plugin(id.0);

                for item in results {
                    ui.result(&item);
                }
            }
        }

        ui.epilog();
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
