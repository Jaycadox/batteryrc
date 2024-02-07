use std::{error::Error, process::Command};

use directories::ProjectDirs;
use systemstat::{Duration, Platform};

struct ShellCommand {
    name: String,
    args: Vec<String>,
}

impl TryFrom<&str> for ShellCommand {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let Ok(value) = shellwords::split(value) else {
            eprintln!("Unable to parse shell command");
            return Err("Failed shell command parse".into());
        };
        let Some(name) = value.first() else {
            eprintln!("Could not find name for shell command: {value:?}");
            return Err("Invalid shell command".into());
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

impl Config {
    fn parse_config(config_text: &str) -> Result<Self, Box<dyn Error>> {
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
                                eprintln!(
                                    "Error while parsing shell command for battery state. {e}"
                                );
                                continue;
                            }
                        })
                    }
                    ParseMode::Ac => config.on_ac_cmds.push(match ShellCommand::try_from(x) {
                        Ok(content) => content,
                        Err(e) => {
                            eprintln!("Error while parsing shell command for AC state. {e}");
                            continue;
                        }
                    }),
                    ParseMode::None => {
                        eprintln!("Attempted to specify command without valid parse mode");
                        continue;
                    }
                },
            }
        }

        Ok(config)
    }

    pub fn try_new() -> Result<Config, Box<dyn Error>> {
        if let Some(dirs) = ProjectDirs::from("io.github", "Jaycadox", "batteryrc") {
            let config = dirs.config_dir();
            if !std::path::Path::exists(config) {
                std::fs::create_dir_all(config)?; // I suppose if the directory fails to be
                                                  // created, the user has bigger problems.
            }
            let config = config.join(".batteryrc");
            let config_contents = std::fs::read_to_string(config)?;

            return Self::parse_config(&config_contents);
        }
        Err("unable to find config path".into())
    }
}

fn power_status_changed(config: &Config, is_on_ac: bool) -> Result<(), Box<dyn Error>> {
    let commands = if is_on_ac {
        &config.on_ac_cmds
    } else {
        &config.on_bat_cmds
    };

    let mut commands = commands
        .iter()
        .map(|cmd| cmd.to_command())
        .collect::<Vec<_>>();

    dbg!("Battery status changed. On AC = {is_on_ac}.");
    dbg!("Running {} saved commands...", commands.len());
    for command in commands.iter_mut() {
        dbg!("> {}", &command);
        if command.status().is_err() {
            eprintln!("Command ran with a failed exit code");
        };
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let first_arg = args.next();

    if let Some(first_arg) = first_arg {
        match &*first_arg {
            "--help" | "-h" => {
                println!("BatteryRC -- made by jayphen");
                return Ok(());
            }
            _ => {}
        };
    }

    let sys = systemstat::System::new();
    let mut on_ac_power = !sys.on_ac_power()?; // Inverted so the first iteration will run

    if let Err(e) = Config::try_new() {
        // If the config initially fails to load, we probably want to fail quickly
        eprintln!("Failed to load initial configuration!");
        eprintln!("{e}");
        return Ok(());
    }

    loop {
        let Ok(now_on_ac_power) = sys.on_ac_power() else {
            eprintln!("Failed to retrieve battery status");
            continue;
        };

        if now_on_ac_power != on_ac_power {
            let Ok(config) = Config::try_new() else {
                eprintln!("Failed to parse shell configuration");
                continue;
            };
            power_status_changed(&config, now_on_ac_power)?;
        }

        on_ac_power = now_on_ac_power;
        std::thread::sleep(Duration::from_secs(1));
    }
}
