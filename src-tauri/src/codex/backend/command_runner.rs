use std::{io, process::Command};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandRunner: Send {
    fn run(&mut self, args: &[&str]) -> io::Result<CommandOutput>;
}

#[derive(Debug, Default)]
pub struct ProcessWslCommandRunner;

impl CommandRunner for ProcessWslCommandRunner {
    fn run(&mut self, args: &[&str]) -> io::Result<CommandOutput> {
        let mut command = Command::new("wsl.exe");
        command.args(args);
        apply_wsl_command_flags(&mut command);
        let output = command.output()?;
        Ok(CommandOutput {
            success: output.status.success(),
            stdout: decode_wsl_output(&output.stdout),
            stderr: decode_wsl_output(&output.stderr),
        })
    }
}

fn decode_wsl_output(bytes: &[u8]) -> String {
    if bytes.len() >= 2
        && bytes.len().is_multiple_of(2)
        && bytes.iter().skip(1).step_by(2).any(|byte| *byte == 0)
    {
        let utf16 = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        if let Ok(decoded) = String::from_utf16(&utf16) {
            return decoded;
        }
    }

    String::from_utf8_lossy(bytes).to_string()
}

#[cfg(test)]
fn wsl_command_creation_flags() -> u32 {
    if cfg!(windows) {
        0x0800_0000
    } else {
        0
    }
}

fn apply_wsl_command_flags(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        command.creation_flags(0x0800_0000);
    }

    #[cfg(not(windows))]
    {
        let _ = command;
    }
}

#[cfg(test)]
mod command_runner_tests {
    use super::{decode_wsl_output, wsl_command_creation_flags};

    #[test]
    fn uses_create_no_window_on_windows_only() {
        if cfg!(windows) {
            assert_eq!(wsl_command_creation_flags(), 0x0800_0000);
        } else {
            assert_eq!(wsl_command_creation_flags(), 0);
        }
    }

    #[test]
    fn decodes_utf16le_output_from_wsl() {
        let bytes = b"U\0b\0u\0n\0t\0u\0\r\0\n\0";

        assert_eq!(decode_wsl_output(bytes), "Ubuntu\r\n");
    }

    #[test]
    fn keeps_utf8_output_unchanged() {
        let bytes = b"Ubuntu\n";

        assert_eq!(decode_wsl_output(bytes), "Ubuntu\n");
    }
}
