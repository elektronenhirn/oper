use app_dirs::*;
use std::fs::read_to_string;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
const APP_INFO: AppInfo = AppInfo {
    name: "oper",
    author: "Florian Bramer",
};
const DEFAULT_CONFIG: &str = r#"
# Custom command section:
#
# You can map keys to custom commands. These commands are
# executed with disconnected stdin/stdout pipes (to avoid
# interference with oper's UI). If you want to execute
# a shell command, wrap the command into a new terminal process.
# The args field allows substitution of {} with the ID of the
# currently selected commit.

# Start gitk whenever 'i' is pressed, the current selected commit
# will be selected in gitk then.
[[custom_command]]
key = "i"
executable = "gitk"
args = "--select-commit={}"

# Execute git show in a seperate terminal window
[[custom_command]]
key = "d"
executable = "gnome-terminal"
args = "-- git show {}"
"#;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub custom_command: Vec<CustomCommand>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CustomCommand {
    pub key: char,
    pub executable: String,
    pub args: Option<String>,
}

impl Config {
    #[cfg(test)]
    pub fn new() -> Config {
        Config {
            custom_command: vec![],
        }
    }
}

impl CustomCommand {
    #[cfg(test)]
    pub fn new(key: char, executable: String, args: Option<String>) -> CustomCommand {
        CustomCommand {
            key,
            executable,
            args,
        }
    }
}

fn config_file() -> PathBuf {
    let folder = app_root(AppDataType::UserConfig, &APP_INFO)
        .expect("Failed to access oper's config folder");
    folder.join("config.toml")
}

pub fn read() -> Config {
    let config_file = config_file();

    //create default config file?
    if !config_file.is_file() {
        std::fs::write(&config_file, DEFAULT_CONFIG).expect("Failed to write oper's config file");
    }

    match read_to_string(&config_file) {
        Ok(content) => match deserialize(&content) {
            Ok(config) => config,
            Err(e) => panic!("Error parsing config file {:?}: {}", &config_file, e),
        },
        Err(e) => panic!("Error reading config file {:?}: {}", &config_file, e),
    }
}

fn deserialize(content: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(content)
}

#[cfg(test)]
fn serialize(config: &Config) -> String {
    toml::to_string(config).unwrap()
}

#[test]
fn test_serialize_deserialze() {
    let mut config = Config::new();
    config.custom_command = vec![
        CustomCommand::new(
            'i',
            "gitk".to_string(),
            Some("--select-commit={}".to_string()),
        ),
        CustomCommand::new(
            'd',
            "gnome-terminal".to_string(),
            Some("-- git show {}".to_string()),
        ),
    ];

    assert_eq!(deserialize(&serialize(&config)).unwrap(), config);
}

#[ignore]
#[test]
fn test_config_file() {
    assert_eq!(config_file(), PathBuf::from(""));
}

#[test]
fn test_parse_default_config() {
    let mut shall_config = Config::new();
    shall_config.custom_command = vec![
        CustomCommand::new(
            'i',
            "gitk".to_string(),
            Some("--select-commit={}".to_string()),
        ),
        CustomCommand::new(
            'd',
            "gnome-terminal".to_string(),
            Some("-- git show {}".to_string()),
        ),
    ];

    let is_config = deserialize(DEFAULT_CONFIG).unwrap();
    assert_eq!(shall_config, is_config);
}
