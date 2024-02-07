use std::path::PathBuf;

use crate::shell_command::*;
use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use tracing::error;

pub struct Config {
    pub on_ac_cmds: Vec<ShellCommand>,
    pub on_bat_cmds: Vec<ShellCommand>,
}

pub enum PathType {
    Config,
    Logs,
}

impl Config {
    fn parse_config(config_text: &str) -> Result<Self> {
        let mut config = Self {
            on_ac_cmds: vec![],
            on_bat_cmds: vec![],
        };

        enum ParseMode {
            None,
            Battery,
            Ac,
        }

        let mut parse_mode = ParseMode::None;
        let lines = config_text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<&str>>();

        for line in lines {
            match &*line.to_lowercase() {
                "@battery" => parse_mode = ParseMode::Battery,
                "@ac" => parse_mode = ParseMode::Ac,
                x => match parse_mode {
                    ParseMode::Battery => {
                        config.on_bat_cmds.push(match ShellCommand::try_from(x) {
                            Ok(content) => content,
                            Err(e) => {
                                error!("Error while parsing shell command for battery state. {e}");
                                continue;
                            }
                        })
                    }
                    ParseMode::Ac => config.on_ac_cmds.push(match ShellCommand::try_from(x) {
                        Ok(content) => content,
                        Err(e) => {
                            error!("Error while parsing shell command for AC state. {e}");
                            continue;
                        }
                    }),
                    ParseMode::None => {
                        error!("Attempted to specify command without valid parse mode");
                        continue;
                    }
                },
            }
        }

        Ok(config)
    }

    pub fn try_new() -> Result<Config> {
        if let Ok(path) = Self::get_path(PathType::Config) {
            let config_contents = std::fs::read_to_string(path)?;
            return Self::parse_config(&config_contents);
        }
        Err(anyhow!("Unable to find config path"))
    }

    pub fn get_path(ptype: PathType) -> Result<PathBuf> {
        if let Some(dirs) = ProjectDirs::from("io.github", "Jaycadox", "batteryrc") {
            let mut config = dirs.config_dir().to_path_buf();
            if !std::path::Path::exists(&config) {
                std::fs::create_dir_all(&config)?; // I suppose if the directory fails to be
                                                   // created, the user has bigger problems.
            }

            match ptype {
                PathType::Config => {
                    config = config.join(".batteryrc");
                }
                PathType::Logs => {
                    config = config.join("logs");
                    std::fs::create_dir_all(&config)?;
                }
            };

            return Ok(config);
        }
        Err(anyhow!("Unable to find config path"))
    }
}

#[test]
fn config_parse() {
    let config_str = "@ac\ntestcmd 1\ncmdtest 2\n@battery\nbattest 1\nbattest 2\nbattest 3";
    let config = Config::parse_config(config_str).unwrap();

    // Test number of commands on AC and battery
    assert_eq!(config.on_ac_cmds.len(), 2);
    assert_eq!(config.on_bat_cmds.len(), 3);

    // Test AC commands
    assert_eq!(config.on_ac_cmds[0].name, "testcmd");
    assert_eq!(config.on_ac_cmds[0].args, vec!["1".to_string()]);
    assert_eq!(config.on_ac_cmds[1].name, "cmdtest");
    assert_eq!(config.on_ac_cmds[1].args, vec!["2".to_string()]);

    // Test battery commands
    assert_eq!(config.on_bat_cmds[0].name, "battest");
    assert_eq!(config.on_bat_cmds[0].args, vec!["1".to_string()]);
    assert_eq!(config.on_bat_cmds[1].name, "battest");
    assert_eq!(config.on_bat_cmds[1].args, vec!["2".to_string()]);
    assert_eq!(config.on_bat_cmds[2].name, "battest");
    assert_eq!(config.on_bat_cmds[2].args, vec!["3".to_string()]);
}
