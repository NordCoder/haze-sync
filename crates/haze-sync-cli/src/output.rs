use crate::config::OutputFormat;
use serde::Serialize;
use std::process::ExitCode;

/// Process exit-code categories used by the CLI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CliExitCode {
    /// Command parsed and completed successfully.
    Success,
    /// Runtime command failed after parsing succeeded.
    RuntimeError,
    /// Command-line usage or parse error.
    UsageError,
}

impl CliExitCode {
    /// Numeric process exit code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::RuntimeError => 1,
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

    /// Safe runtime error output.
    #[must_use]
    pub fn runtime_error(stderr: impl Into<String>) -> Self {
        Self {
            exit_code: CliExitCode::RuntimeError,
            stdout: String::new(),
            stderr: stderr.into(),
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

    /// Apply the selected stable output envelope without changing the exit code.
    #[must_use]
    pub fn formatted(self, format: OutputFormat, command: &str) -> Self {
        if format == OutputFormat::Human {
            return self;
        }

        let envelope = JsonEnvelope {
            schema: "haze-sync.cli.output.v1",
            command,
            ok: self.exit_code == CliExitCode::Success,
            exit_code: self.exit_code.code(),
            stdout: &self.stdout,
            stderr: &self.stderr,
        };
        let stdout = serde_json::to_string(&envelope)
            .expect("serializing the bounded CLI output envelope cannot fail");
        Self {
            exit_code: self.exit_code,
            stdout,
            stderr: String::new(),
        }
    }
}

#[derive(Serialize)]
struct JsonEnvelope<'a> {
    schema: &'static str,
    command: &'a str,
    ok: bool,
    exit_code: u8,
    stdout: &'a str,
    stderr: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_envelope_is_deterministic_and_preserves_exit_code() {
        let output = CliOutput {
            exit_code: CliExitCode::RuntimeError,
            stdout: "safe partial result".to_owned(),
            stderr: "safe failure".to_owned(),
        }
        .formatted(OutputFormat::Json, "preflight");

        assert_eq!(output.exit_code, CliExitCode::RuntimeError);
        assert!(output.stderr.is_empty());
        assert_eq!(
            output.stdout,
            r#"{"schema":"haze-sync.cli.output.v1","command":"preflight","ok":false,"exit_code":1,"stdout":"safe partial result","stderr":"safe failure"}"#
        );
    }

    #[test]
    fn human_output_is_unchanged() {
        let output = CliOutput::success("ready");
        assert_eq!(
            output.clone().formatted(OutputFormat::Human, "preflight"),
            output
        );
    }
}
