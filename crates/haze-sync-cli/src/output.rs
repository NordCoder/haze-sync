use std::process::ExitCode;

/// Process exit-code categories used by the CLI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CliExitCode {
    /// Command parsed and completed successfully.
    Success,
    /// Command-line usage or parse error.
    UsageError,
}

impl CliExitCode {
    /// Numeric process exit code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::UsageError => 2,
        }
    }

    /// Convert to the standard library process exit-code type.
    #[must_use]
    pub fn into_exit_code(self) -> ExitCode {
        ExitCode::from(self.code())
    }
}

/// Rendered command result separated into stdout, stderr, and exit-code channel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliOutput {
    pub exit_code: CliExitCode,
    pub stdout: String,
    pub stderr: String,
}

impl CliOutput {
    /// Successful read-only command output.
    #[must_use]
    pub fn success(stdout: impl Into<String>) -> Self {
        Self {
            exit_code: CliExitCode::Success,
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    /// Safe parse/usage error output.
    #[must_use]
    pub fn usage_error(stderr: impl Into<String>) -> Self {
        Self {
            exit_code: CliExitCode::UsageError,
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }
}
