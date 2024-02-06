use std::{error::Error, process::Command};

use directories::ProjectDirs;
use systemstat::{Duration, Platform};

struct Config {
    on_ac_cmds: Vec<String>,
    on_bat_cmds: Vec<String>,
}

impl Config {
    fn parse_config(
        config: &str,
        ac_cmds: &mut Vec<String>,
        bat_cmds: &mut Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        enum ParseMode {
            None,
            Battery,
            Ac,
        }

        let mut parse_mode = ParseMode::None;
        let lines = config.lines().map(|l| l.trim()).collect::<Vec<&str>>();

        for line in lines {
            match line {
                "@battery" => parse_mode = ParseMode::Battery,
                "@ac" => parse_mode = ParseMode::Ac,
                x => match parse_mode {
                    ParseMode::Battery => bat_cmds.push(x.to_string()),
                    ParseMode::Ac => ac_cmds.push(x.to_string()),
                    ParseMode::None => {
                        return Err("Attempted to specify command without valid parse mode".into())
                    }
                },
            }
        }

        Ok(())
    }

    pub fn try_new() -> Result<Config, Box<dyn Error>> {
        if let Some(dirs) = ProjectDirs::from("io.github", "Jaycadox", "batteryrc") {
            let config = dirs.config_dir();
            if !std::path::Path::exists(config) {
                std::fs::create_dir_all(config)?;
            }
            let config = config.join(".batteryrc");
            let config_contents = std::fs::read_to_string(config)?;

            let mut ac_cmds = vec![];
            let mut bat_cmds = vec![];
            Self::parse_config(&config_contents, &mut ac_cmds, &mut bat_cmds)?;

            return Ok(Self {
                on_ac_cmds: ac_cmds,
                on_bat_cmds: bat_cmds,
            });
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

    for command in commands {
        if command.trim().is_empty() {
            continue;
        }

        let command_parts = shellwords::split(command)?;
        let mut command = Command::new(command_parts.first().expect("Command does not have name"));
        if command_parts.len() > 1 {
            command.args(&command_parts[1..]);
        }

        let _ = command.status()?;
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let sys = systemstat::System::new();
    let mut on_ac_power = !sys.on_ac_power()?; // Inverted so the first iteration will run

    loop {
        let now_on_ac_power = sys.on_ac_power()?;

        if now_on_ac_power != on_ac_power {
            let config = Config::try_new()?;
            power_status_changed(&config, now_on_ac_power)?;
        }

        on_ac_power = now_on_ac_power;
        std::thread::sleep(Duration::from_secs(1));
    }
}
