use anyhow::Result;
use config::Config;
use tracing::{debug, error, info, trace};
use tracing_subscriber::layer::SubscriberExt;

use systemstat::{Duration, Platform};

use crate::config::PathType;

mod config;
mod shell_command;

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

fn setup_tracing() -> Result<()> {
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
    Ok(())
}

fn main() -> Result<()> {
    if setup_tracing().is_err() {
        eprintln!("Failed to start tracing logging");
    }

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
