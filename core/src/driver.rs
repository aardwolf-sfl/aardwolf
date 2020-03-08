use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use crate::api::Api;
use crate::config::{Config, LoadConfigError};
use crate::logger::Logger;
use crate::plugins::{
    collect_bb::CollectBb, invariants::Invariants, irrelevant::Irrelevant, prob_graph::ProbGraph,
    sbfl::Sbfl, AardwolfPlugin, IrrelevantItems, Results,
};
use crate::raw::Data;
use crate::ui::CliUi;

// TARGET_FILE (program code, usually preprocessed)
//     | program analysis and instrumentation
//     v
// INSTR_FILE
//     | final compilation
//     v
// EXEC_FILE
//     | execution --> TRACE_FILE
//     v
// RESULT_FILE (test results)
pub const TARGET_FILE: &'static str = "aard.target";
pub const INSTR_FILE: &'static str = "aard.instr";
pub const EXEC_FILE: &'static str = "aard.exec";
pub const TRACE_FILE: &'static str = "aard.trace";
pub const RESULT_FILE: &'static str = "aard.result";
pub const LOG_FILE: &'static str = "aard.log";

pub const DEFAULT_CONFIG_FILE: &'static str = ".aardwolf.yml";
pub const DEFAULT_SHELL: &'static str = "bash";

pub struct DriverPaths {
    pub output_dir: PathBuf,
    pub work_dir: PathBuf,
    pub runtime_lib: PathBuf,
    pub frontend: PathBuf,
    pub target_file: PathBuf,
    pub instr_file: PathBuf,
    pub exec_file: PathBuf,
    pub analysis_file: PathBuf,
    pub trace_file: PathBuf,
    pub result_file: PathBuf,
}

impl DriverPaths {
    pub fn new<P1: AsRef<Path>, P2: AsRef<Path>>(
        config: &Config,
        config_path: P1,
        args: &DriverArgs<P2>,
    ) -> Self {
        let output_dir = config_path.as_ref().join(&config.output_dir);

        let target_file = output_dir.join(TARGET_FILE);

        let mut analysis_file = target_file.clone();
        let mut analysis_filename = analysis_file.file_name().unwrap().to_os_string();
        analysis_filename.push(".aard");
        analysis_file.set_file_name(analysis_filename);

        // TODO: Allow to override the files (e.g., llvm linker needs .bc extensions to process the files).
        DriverPaths {
            target_file,
            instr_file: output_dir.join(INSTR_FILE),
            exec_file: output_dir.join(EXEC_FILE),
            analysis_file,
            trace_file: output_dir.join(TRACE_FILE),
            result_file: output_dir.join(RESULT_FILE),
            output_dir,
            work_dir: config_path.as_ref().to_path_buf(),
            runtime_lib: args
                .runtime_path
                .as_ref()
                // FIXME: Handle canonicalization error.
                .canonicalize()
                .unwrap()
                .to_path_buf(),
            frontend: args
                .frontend_path
                .as_ref()
                // FIXME: Handle canonicalization error.
                .map(|path| path.as_ref().canonicalize().unwrap().to_path_buf())
                .unwrap_or_default(),
        }
    }
}

pub struct DriverArgs<P: AsRef<Path>> {
    runtime_path: P,
    config_path: Option<P>,
    frontend_path: Option<P>,
}

impl<P: AsRef<Path>> DriverArgs<P> {
    pub fn new(runtime_path: P) -> Self {
        DriverArgs {
            runtime_path: runtime_path,
            config_path: None,
            frontend_path: None,
        }
    }

    pub fn with_config_path(self, config_path: Option<P>) -> Self {
        Self {
            config_path,
            ..self
        }
    }

    pub fn with_frontend_path(self, frontend_path: Option<P>) -> Self {
        Self {
            frontend_path,
            ..self
        }
    }
}

// Process localization plugins as they are defined in the config.
// Implement ordering by its index.
#[derive(Eq)]
struct LocalizationId<'data>(&'data str, usize);

impl<'data> LocalizationId<'data> {
    pub fn new(name: &'data str, index: usize) -> Self {
        LocalizationId(name, index)
    }
}

impl<'data> Ord for LocalizationId<'data> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.1.cmp(&other.1)
    }
}

impl<'data> PartialOrd for LocalizationId<'data> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'data> PartialEq for LocalizationId<'data> {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

pub struct Driver;

impl Driver {
    pub fn run<P: AsRef<Path>>(args: &DriverArgs<P>) {
        let (config, config_path) = Self::load_config(args.config_path.as_ref()).unwrap();
        let driver_paths = DriverPaths::new(&config, &config_path, args);

        fs::create_dir_all(&driver_paths.output_dir).unwrap();

        let mut logger = Logger::new(driver_paths.output_dir.join(LOG_FILE));
        logger.info("config file loaded");

        let script_handle = logger.perf("run script");
        Self::run_script(&config, &driver_paths).unwrap();
        script_handle.stop();

        let data_handle = logger.perf("load data");
        let data = Self::load_data(&driver_paths);
        data_handle.stop();

        let api = Api::new(data).unwrap();

        let init_handle = logger.perf("init plugins");
        let plugins = Self::init_plugins(&config, &api);
        init_handle.stop();

        let results = Self::run_loc(&config, &api, &plugins, &mut logger);

        let display_handle = logger.perf("display results");
        Self::display_results(&config, &api, results);
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
            .env("WORK_DIR", &driver_paths.work_dir)
            .env("RUNTIME_LIB", &driver_paths.runtime_lib)
            .env("FRONTEND", &driver_paths.frontend)
            .env("TARGET_FILE", &driver_paths.target_file)
            .env("INSTR_FILE", &driver_paths.instr_file)
            .env("EXEC_FILE", &driver_paths.exec_file)
            .env("ANALYSIS_FILE", &driver_paths.analysis_file)
            .env("TRACE_FILE", &driver_paths.trace_file)
            .env("RESULT_FILE", &driver_paths.result_file)
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        Ok(())
    }

    fn load_data(driver_paths: &DriverPaths) -> Data {
        let mut static_file = BufReader::new(File::open(&driver_paths.analysis_file).unwrap());
        let mut dynamic_file = BufReader::new(File::open(&driver_paths.trace_file).unwrap());
        let mut test_file = BufReader::new(File::open(&driver_paths.result_file).unwrap());

        Data::parse(&mut static_file, &mut dynamic_file, &mut test_file).unwrap()
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

    fn init_plugins<'data>(
        config: &'data Config,
        api: &'data Api<'data>,
    ) -> Vec<(&'data str, Box<dyn AardwolfPlugin>)> {
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

    fn run_loc<'data>(
        config: &'data Config,
        api: &'data Api<'data>,
        plugins: &'data Vec<(&'data str, Box<dyn AardwolfPlugin>)>,
        logger: &mut Logger,
    ) -> BTreeMap<LocalizationId<'data>, Results<'data>> {
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

    fn display_results<'data>(
        config: &'data Config,
        api: &'data Api<'data>,
        results: BTreeMap<LocalizationId<'data>, Results<'data>>,
    ) {
        let mut ui = CliUi::new(api).unwrap();

        for (id, results) in results.into_iter() {
            if Self::should_display(config, &id) {
                ui.plugin(id.0);

                for item in results {
                    ui.result(&item);
                }
            }
        }
    }

    fn n_results<'data>(config: &'data Config, id: &LocalizationId<'data>) -> usize {
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

    fn should_display<'data>(config: &'data Config, id: &LocalizationId<'data>) -> bool {
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
