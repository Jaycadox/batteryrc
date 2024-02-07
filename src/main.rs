use anyhow::{anyhow, Result};
use std::{path::PathBuf, process::Command};
use tracing::{debug, error, info, trace};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

use directories::ProjectDirs;
use systemstat::{Duration, Platform};

struct ShellCommand {
    name: String,
    args: Vec<String>,
}

impl TryFrom<&str> for ShellCommand {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        let Ok(value) = shellwords::split(value) else {
            error!("Unable to parse shell command");
            return Err(anyhow!("Unable to parse shell command"));
        };
        let Some(name) = value.first() else {
            error!("Could not find name for shell command: {value:?}");
            return Err(anyhow!("Invalid shell command"));
        };
        let name = name.to_string();

        let args = value.get(1..).map(|x| x.to_vec()).unwrap_or_default();

        Ok(Self { name, args })
    }
}

impl ShellCommand {
    fn to_command(&self) -> Command {
        let command_name = &self.name;
        let command_args = &self.args;

        let mut command = Command::new(command_name);
        if !command_args.is_empty() {
            command.args(command_args);
        }

        command
    }
}

struct Config {
    on_ac_cmds: Vec<ShellCommand>,
    on_bat_cmds: Vec<ShellCommand>,
}

enum PathType {
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

fn power_status_changed(config: &Config, is_on_ac: bool) -> Result<()> {
    let commands = if is_on_ac {
        &config.on_ac_cmds
    } else {
        &config.on_bat_cmds
    };

    let mut commands = commands
        .iter()
        .map(|cmd| cmd.to_command())
        .collect::<Vec<_>>();

    info!("Battery status changed. On AC = {is_on_ac}.");
    debug!("Running {} saved commands...", commands.len());
    for command in commands.iter_mut() {
        trace!("> {:?}", &command);
        if command.status().is_err() {
            error!("Command ran with a failed exit code");
        };
    }

    Ok(())
}

fn main() -> Result<()> {
    let file_appender = Config::get_path(PathType::Logs)
        .ok()
        .map(|logs| tracing_appender::rolling::daily(logs, "batteryrc.log"));

    let file_layer = file_appender
        .map(|file_appender| tracing_subscriber::fmt::Layer::new().with_writer(file_appender));

    let sub = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        //.with(EnvFilter::from_default_env())
        .with(file_layer);

    tracing::subscriber::set_global_default(sub)?;

    let mut args = std::env::args().skip(1);
    let first_arg = args.next();

    if let Some(first_arg) = first_arg {
        match &*first_arg {
            "--help" | "-h" => {
                println!("BatteryRC -- made by jayphen");
                println!(
                    "Looking for config in: {}",
                    match Config::get_path(PathType::Config) {
                        Ok(path) => path
                            .to_str()
                            .to_owned()
                            .unwrap_or("Unable to display path")
                            .to_string(),
                        Err(e) => format!("Unable to find path: {}", e),
                    }
                );
                return Ok(());
            }
            _ => {}
        };
    }

    let sys = systemstat::System::new();
    let mut on_ac_power = !sys.on_ac_power()?; // Inverted so the first iteration will run

    if let Err(e) = Config::try_new() {
        // If the config initially fails to load, we probably want to fail quickly
        error!("Failed to load initial configuration!");
        error!("{e}");
        return Ok(());
    }

    info!("BatteryRC started.");

    loop {
        let Ok(now_on_ac_power) = sys.on_ac_power() else {
            error!("Failed to retrieve battery status");
            continue;
        };

        if now_on_ac_power != on_ac_power {
            let Ok(config) = Config::try_new() else {
                error!("Failed to parse shell configuration");
                continue;
            };
            power_status_changed(&config, now_on_ac_power)?;
        }

        on_ac_power = now_on_ac_power;
        std::thread::sleep(Duration::from_secs(1));
    }
}
