//! Aardwolf configuration.
//!
//! The configuration is stored (by default) in `.aardwolf.yml` file in the
//! project root. There are currently four options that can be specified.
//!
//! * `script` **(required)** -- The script which is executed before the
//!   localization process begins. Its purpose is to compile the sources using
//!   an Aardwolf frontend and then run the test suite. After it is completed,
//!   all data (static analysis, runtime and test results) are expected to be
//!   generated on expected locations. The script is list of lines at the
//!   moment, but more flexible approach will be implemented in the future
//!   (script file, platform-specific scripts, etc.). If the script execution,
//!   then the whole Aardwolf process is terminated. For ignoring expected
//!   errors, use the shell features like `|| true`.
//!
//! * `plugins` **(required)** -- List of plugins which will be used in the
//!   localization process. A custom name can be given to each plugin
//!   "instance". Plugins are usually customizable via their options.
//!
//! * `output_dir` (optional, default: `.aardwolf`) -- Path to a directory where
//!   Aardwolf will store all its data. The path is relative to the project root
//!   where the configuration file was found.
//!
//! * `n_results` (optional, default: `10`) -- Number of predicated items that
//!   will be displayed to the user from each plugin. If set to `0`, then the
//!   limit will be ignored. This option can be also set to each plugin
//!   individually.
//!
//! # Examples
//!
//! ```yml
//! script:
//!   # In Python frontend, all the machinery is done in test files.
//!   - pytest || true
//!
//! plugins:
//!   - sbfl: D* Spectrum
//!     options:
//!       metric: ochiai
//!   - prob-graph: Probabilistic Dependence
//!   - invariants: Likely Invariants
//! ```

use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{self, prelude::*};
use std::path::{Path, PathBuf};

use yaml_rust::{ScanError, Yaml, YamlLoader};

pub const DEFAULT_OUTPUT_DIR: &'static str = ".aardwolf";
pub const DEFAULT_N_RESULTS: usize = 10;

/// Plugin instance inside the configuration.
///
/// It contains the plugin identifier, optionally a custom name and set of
/// options.
#[derive(Debug)]
pub struct Plugin {
    pub id: String,
    pub name: Option<String>,
    pub opts: HashMap<String, Yaml>,
}

impl Plugin {
    /// Creates plugin with given identifier with default name and empty options.
    pub fn new<T: Into<String>>(id: T) -> Self {
        Plugin {
            id: id.into(),
            name: None,
            opts: HashMap::new(),
        }
    }

    /// Creates plugin with given identifier and custom name.
    pub fn with_name<T1: Into<String>, T2: Into<String>>(id: T1, name: T2) -> Self {
        Self::with_name_and_opts(id, name, HashMap::new())
    }

    /// Creates plugin with given identifier, custom name and the set of
    /// options.
    pub fn with_name_and_opts<T1: Into<String>, T2: Into<String>>(
        id: T1,
        name: T2,
        opts: HashMap<String, Yaml>,
    ) -> Self {
        Plugin {
            id: id.into(),
            name: Some(name.into()),
            opts,
        }
    }

    /// Gets the identification of the plugin inside configuration. Custom name
    /// has higher priority than original identifier.
    pub fn id(&self) -> &str {
        match &self.name {
            Some(name) => name,
            None => &self.id,
        }
    }
}

/// Configuration structure.
#[derive(Debug)]
pub struct Config {
    /// Script lines.
    pub script: Vec<String>,
    /// Output directory.
    pub output_dir: PathBuf,
    /// Number of results to display.
    pub n_results: usize,
    /// Collection of plugins.
    pub plugins: Vec<Plugin>,
}

#[derive(Debug)]
pub enum LoadConfigError {
    Io(io::Error),
    Yaml(ScanError),
    Invalid(String),
    UnknownOption(String),
    NotFound,
}

impl fmt::Display for LoadConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadConfigError::Io(error) => write!(f, "{}", error),
            LoadConfigError::Yaml(error) => write!(f, "invalid configuration syntax: {}", error),
            LoadConfigError::Invalid(error) => write!(f, "invalid configuration format: {}", error),
            LoadConfigError::UnknownOption(error) => write!(f, "unknown configuration option: {}", error),
            LoadConfigError::NotFound => write!(f, "configuration not found"),
        }
    }
}

impl Config {
    /// Loads the configuration from given file. If optional items are not
    /// specified, default values are used.
    pub fn load_from_file<P: AsRef<Path>>(filepath: P) -> Result<Self, LoadConfigError> {
        let mut file = File::open(&filepath).map_err(|err| LoadConfigError::Io(err))?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|err| LoadConfigError::Io(err))?;

        let yaml = YamlLoader::load_from_str(&content).map_err(|err| LoadConfigError::Yaml(err))?;

        let config = yaml
            .get(0)
            .ok_or(LoadConfigError::Invalid("Empty file".to_string()))
            .and_then(|item| {
                item.as_hash()
                    .ok_or(LoadConfigError::Invalid("Invalid format".to_string()))
            })?;

        let mut script = Vec::new();
        let mut output_dir = filepath.as_ref().parent().unwrap().join(DEFAULT_OUTPUT_DIR);
        let mut n_results = DEFAULT_N_RESULTS;
        let mut plugins = Vec::new();

        for (key, value) in config {
            match key
                .as_str()
                .ok_or(LoadConfigError::Invalid("Invalid format".to_string()))?
            {
                "script" => {
                    for line in value
                        .as_vec()
                        .ok_or(LoadConfigError::Invalid("Invalid format".to_string()))?
                    {
                        script.push(
                            line.as_str()
                                .ok_or(LoadConfigError::Invalid("Invalid format".to_string()))?
                                .to_string(),
                        );
                    }
                }
                "output_dir" => {
                    output_dir = filepath.as_ref().parent().unwrap().join(
                        value
                            .as_str()
                            .ok_or(LoadConfigError::Invalid("Invalid format".to_string()))?,
                    );
                }
                "n_results" => {
                    n_results = value
                        .as_i64()
                        .ok_or(LoadConfigError::Invalid("Invalid format".to_string()))?
                        as usize;
                }
                "plugins" => {
                    for plugin in value
                        .as_vec()
                        .ok_or(LoadConfigError::Invalid("Invalid format".to_string()))?
                    {
                        match plugin {
                            Yaml::String(id) => plugins.push(Plugin::new(id)),
                            Yaml::Hash(hash) => {
                                if let Some(item) = hash
                                    .iter()
                                    .find(|(item_key, _)| item_key.as_str() != Some("options"))
                                {
                                    match item {
                                        (Yaml::String(id), Yaml::Null) => {
                                            plugins.push(Plugin::new(id))
                                        }
                                        (Yaml::String(id), Yaml::String(name)) => {
                                            if let Some(options) =
                                                hash.iter().find(|(item_key, _)| {
                                                    item_key.as_str() == Some("options")
                                                })
                                            {
                                                let mut plugin_options = HashMap::new();

                                                for (option_key, option_value) in options
                                                    .1
                                                    .as_hash()
                                                    .ok_or(LoadConfigError::Invalid(
                                                        "Invalid format".to_string(),
                                                    ))?
                                                {
                                                    plugin_options.insert(
                                                        option_key
                                                            .as_str()
                                                            .ok_or(LoadConfigError::Invalid(
                                                                "Invalid format".to_string(),
                                                            ))?
                                                            .to_string(),
                                                        option_value.clone(),
                                                    );
                                                }

                                                plugins.push(Plugin::with_name_and_opts(
                                                    id,
                                                    name,
                                                    plugin_options,
                                                ));
                                            } else {
                                                plugins.push(Plugin::with_name(id, name));
                                            }
                                        }
                                        _ => {
                                            return Err(LoadConfigError::Invalid(
                                                "Invalid format".to_string(),
                                            ))
                                        }
                                    }
                                } else {
                                    return Err(LoadConfigError::Invalid(
                                        "Invalid format".to_string(),
                                    ));
                                }
                            }
                            _ => {
                                return Err(LoadConfigError::Invalid("Invalid format".to_string()))
                            }
                        }
                    }
                }
                option => return Err(LoadConfigError::UnknownOption(option.to_string())),
            }
        }

        Ok(Config {
            script,
            output_dir,
            n_results,
            plugins,
        })
    }
}
