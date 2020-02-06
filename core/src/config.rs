use std::collections::HashMap;
use std::fs::File;
use std::io::{self, prelude::*};
use std::path::{Path, PathBuf};

use yaml_rust::{ScanError, Yaml, YamlLoader};

#[derive(Debug)]
pub struct Plugin {
    pub id: String,
    pub name: Option<String>,
    pub opts: HashMap<String, Yaml>,
}

impl Plugin {
    pub fn new<T: Into<String>>(id: T) -> Self {
        Plugin {
            id: id.into(),
            name: None,
            opts: HashMap::new(),
        }
    }

    pub fn with_name<T1: Into<String>, T2: Into<String>>(id: T1, name: T2) -> Self {
        Self::with_name_and_opts(id, name, HashMap::new())
    }

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
}

#[derive(Debug)]
pub struct Config {
    pub script: Vec<String>,
    pub output_dir: PathBuf,
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

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(filepath: P) -> Result<Self, LoadConfigError> {
        let mut file = File::open(filepath).map_err(|err| LoadConfigError::Io(err))?;
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
        let mut output_dir = PathBuf::new();
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
                    output_dir = value
                        .as_str()
                        .ok_or(LoadConfigError::Invalid("Invalid format".to_string()))?
                        .into();
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
            plugins,
        })
    }
}
