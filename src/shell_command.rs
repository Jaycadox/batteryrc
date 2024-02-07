use anyhow::{anyhow, Result};
use std::process::Command;
use tracing::error;

#[derive(PartialEq, Debug)]
pub struct ShellCommand {
    pub name: String,
    pub args: Vec<String>,
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
    pub fn to_command(&self) -> Command {
        let command_name = &self.name;
        let command_args = &self.args;

        let mut command = Command::new(command_name);
        if !command_args.is_empty() {
            command.args(command_args);
        }

        command
    }
}

#[test]
fn shell_parse() {
    let command = "testcmd \"first arg\" 1 second third";
    let shell_cmd = ShellCommand::try_from(command);
    assert_eq!(
        shell_cmd.unwrap(),
        ShellCommand {
            name: "testcmd".to_string(),
            args: vec![
                "first arg".to_string(),
                "1".to_string(),
                "second".to_string(),
                "third".to_string()
            ]
        }
    );
}

#[test]
fn shell_command_conversion() {
    let command = "testcmd \"first arg\" 1 second third";
    let shell_cmd = ShellCommand::try_from(command);
    let actual_command = shell_cmd.unwrap().to_command();
    assert_eq!(actual_command.get_program().to_str().unwrap(), "testcmd");
    assert_eq!(
        actual_command
            .get_args()
            .map(|x| x.to_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["first arg", "1", "second", "third"]
    );
}
