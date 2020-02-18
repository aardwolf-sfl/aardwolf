use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use yaml_rust::Yaml;

use crate::api::Api;
use crate::config::{Config, LoadConfigError};
use crate::plugins::{prob_graph::ProbGraph, sbfl::Sbfl, AardwolfPlugin};
use crate::raw::Data;

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

    pub fn with_frontend_path(mut self, frontend_path: Option<P>) -> Self {
        Self {
            frontend_path,
            ..self
        }
    }
}

pub struct Driver;

impl Driver {
    pub fn run<P: AsRef<Path>>(args: &DriverArgs<P>) {
        let (config, config_path) = Self::load_config(args.config_path.as_ref()).unwrap();
        let driver_paths = DriverPaths::new(&config, &config_path, args);

        fs::create_dir_all(&driver_paths.output_dir).unwrap();

        Self::run_script(&config, &driver_paths).unwrap();

        let data = Driver::load_data(&driver_paths);
        let api = Api::new(data);

        for plugin in config.plugins.iter() {
            let name = match &plugin.name {
                Some(name) => name,
                None => &plugin.id,
            };
            match plugin.id.as_str() {
                "sbfl" => Self::run_loc::<Sbfl>(name, &api, &plugin.opts),
                "prob-graph" => Self::run_loc::<ProbGraph>(name, &api, &plugin.opts),
                _ => panic!("Unknown plugin"),
            }
        }
    }

    fn run_loc<'a, P: 'a + AardwolfPlugin>(
        name: &'a str,
        api: &'a Api<'a>,
        opts: &'a HashMap<String, Yaml>,
    ) {
        let plugin = P::init(api, opts).unwrap();
        let mut results = plugin.run_loc(api);

        // Use stable sort to not break plugins which sort the results using another criterion.
        results.sort_by(|lhs, rhs| rhs.cmp(lhs));

        println!("Results for: {}", name);
        for item in results.iter().take(10) {
            println!("{:?}\t{}\t{:?}", item.loc, item.score, item.rationale);
        }
        println!();
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
}
