//! Bounded process configuration and token-source loading.

use crate::config::{
    CliConfig, ConfigError, OutputFormat, ProfileName, ServerUrl, TokenSource,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::OnceLock,
};

const CONFIG_PATH_ENV: &str = "HAZE_SYNC_CONFIG";
const PROFILE_ENV: &str = "HAZE_SYNC_PROFILE";
const SERVER_URL_ENV: &str = "HAZE_SYNC_SERVER_URL";
const OUTPUT_FORMAT_ENV: &str = "HAZE_SYNC_OUTPUT";
const TOKEN_SOURCE_ENV: &str = "HAZE_SYNC_TOKEN_SOURCE";
const MAX_CONFIG_BYTES: usize = 64 * 1024;
const MAX_CONFIG_LINE_BYTES: usize = 4 * 1024;
const MAX_CONFIG_PROFILES: usize = 64;
const MAX_TOKEN_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConfigOverrides {
    config_path: Option<PathBuf>,
    profile: Option<String>,
    server_url: Option<String>,
    output_format: Option<String>,
    token_source: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedInvocation {
    pub command_args: Vec<String>,
    pub overrides: ConfigOverrides,
}

pub fn split_global_options<I, S>(args: I) -> Result<ParsedInvocation, InvocationError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args
        .into_iter()
        .map(|argument| argument.as_ref().to_owned())
        .collect::<Vec<_>>();
    if args.is_empty() {
        args.push("haze-sync".to_owned());
    }

    let program = args.remove(0);
    let mut command_args = vec![program];
    let mut overrides = ConfigOverrides::default();
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        let Some((name, inline_value)) = parse_global_option(argument) else {
            command_args.extend(args[index..].iter().cloned());
            break;
        };

        let value = if let Some(value) = inline_value {
            value.to_owned()
        } else {
            index += 1;
            args.get(index)
                .cloned()
                .ok_or(InvocationError::MissingGlobalOptionValue)?
        };
        if value.is_empty() {
            return Err(InvocationError::MissingGlobalOptionValue);
        }
        set_override(&mut overrides, name, value)?;
        index += 1;
    }

    Ok(ParsedInvocation {
        command_args,
        overrides,
    })
}

fn parse_global_option(argument: &str) -> Option<(&str, Option<&str>)> {
    for name in [
        "--config",
        "--profile",
        "--server-url",
        "--output",
        "--token-source",
    ] {
        if argument == name {
            return Some((name, None));
        }
        if let Some(value) = argument.strip_prefix(name).and_then(|rest| rest.strip_prefix('=')) {
            return Some((name, Some(value)));
        }
    }
    None
}

fn set_override(
    overrides: &mut ConfigOverrides,
    name: &str,
    value: String,
) -> Result<(), InvocationError> {
    let slot = match name {
        "--config" => {
            if overrides.config_path.is_some() {
                return Err(InvocationError::DuplicateGlobalOption);
            }
            overrides.config_path = Some(PathBuf::from(value));
            return Ok(());
        }
        "--profile" => &mut overrides.profile,
        "--server-url" => &mut overrides.server_url,
        "--output" => &mut overrides.output_format,
        "--token-source" => &mut overrides.token_source,
        _ => return Err(InvocationError::UnknownGlobalOption),
    };
    if slot.is_some() {
        return Err(InvocationError::DuplicateGlobalOption);
    }
    *slot = Some(value);
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvocationError {
    MissingGlobalOptionValue,
    DuplicateGlobalOption,
    UnknownGlobalOption,
}

impl fmt::Display for InvocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingGlobalOptionValue => "missing global configuration option value",
            Self::DuplicateGlobalOption => "duplicate global configuration option",
            Self::UnknownGlobalOption => "unknown global configuration option",
        })
    }
}

impl std::error::Error for InvocationError {}

trait SourceProvider {
    fn environment(&self, name: &str) -> Option<String>;
    fn read_file(&self, path: &Path, max_bytes: usize) -> Result<Option<String>, LoadError>;
}

struct ProcessSources;

impl SourceProvider for ProcessSources {
    fn environment(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn read_file(&self, path: &Path, max_bytes: usize) -> Result<Option<String>, LoadError> {
        let mut file = match fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(LoadError::ConfigFileUnavailable),
        };
        let mut bytes = Vec::new();
        file.by_ref()
            .take((max_bytes + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| LoadError::ConfigFileUnavailable)?;
        if bytes.len() > max_bytes {
            return Err(LoadError::ConfigFileTooLarge);
        }
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| LoadError::ConfigFileInvalidEncoding)
    }
}

pub fn load_process_config(overrides: &ConfigOverrides) -> Result<CliConfig, LoadError> {
    load_config(overrides, &ProcessSources)
}

fn load_config(
    overrides: &ConfigOverrides,
    sources: &dyn SourceProvider,
) -> Result<CliConfig, LoadError> {
    let config_path = overrides.config_path.clone().or_else(|| {
        sources
            .environment(CONFIG_PATH_ENV)
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
    });
    let file = match config_path.as_deref() {
        Some(path) => {
            let content = sources
                .read_file(path, MAX_CONFIG_BYTES)?
                .ok_or(LoadError::ConfigFileUnavailable)?;
            parse_config_file(&content)?
        }
        None => ParsedConfigFile::default(),
    };

    let environment_profile = sources.environment(PROFILE_ENV);
    let environment_server_url = sources.environment(SERVER_URL_ENV);
    let environment_output_format = sources.environment(OUTPUT_FORMAT_ENV);
    let environment_token_source = sources.environment(TOKEN_SOURCE_ENV);

    let profile_raw = first_present([
        overrides.profile.as_deref(),
        environment_profile.as_deref(),
        file.default_profile.as_deref(),
        Some("default"),
    ])
    .expect("built-in profile is always present");
    let profile = ProfileName::parse(profile_raw).map_err(LoadError::InvalidValue)?;
    let profile_values = file.profiles.get(profile.as_str());

    let server_url = first_present([
        overrides.server_url.as_deref(),
        environment_server_url.as_deref(),
        profile_values.and_then(|values| values.server_url.as_deref()),
    ])
    .map(ServerUrl::parse)
    .transpose()
    .map_err(LoadError::InvalidValue)?;

    let output_format = first_present([
        overrides.output_format.as_deref(),
        environment_output_format.as_deref(),
        profile_values.and_then(|values| values.output_format.as_deref()),
        Some("human"),
    ])
    .map(OutputFormat::parse)
    .transpose()
    .map_err(LoadError::InvalidValue)?
    .expect("built-in output format is always present");

    let token_source = first_present([
        overrides.token_source.as_deref(),
        environment_token_source.as_deref(),
        profile_values.and_then(|values| values.token_source.as_deref()),
        Some("none"),
    ])
    .map(TokenSource::parse)
    .transpose()
    .map_err(LoadError::InvalidValue)?
    .expect("built-in token source is always present");

    Ok(CliConfig::new(
        profile,
        server_url,
        output_format,
        token_source,
    ))
}

fn first_present<'a, const N: usize>(values: [Option<&'a str>; N]) -> Option<&'a str> {
    values
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ParsedConfigFile {
    default_profile: Option<String>,
    profiles: BTreeMap<String, ProfileValues>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ProfileValues {
    server_url: Option<String>,
    output_format: Option<String>,
    token_source: Option<String>,
}

fn parse_config_file(content: &str) -> Result<ParsedConfigFile, LoadError> {
    let mut parsed = ParsedConfigFile::default();
    let mut current_profile: Option<String> = None;
    let mut seen_keys = BTreeSet::new();

    for raw_line in content.lines() {
        if raw_line.len() > MAX_CONFIG_LINE_BYTES {
            return Err(LoadError::ConfigLineTooLong);
        }
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            if !line.ends_with(']') {
                return Err(LoadError::ConfigSyntax);
            }
            let section = &line[1..line.len() - 1];
            let Some(profile) = section.strip_prefix("profile.") else {
                return Err(LoadError::ConfigUnknownSection);
            };
            let profile = ProfileName::parse(profile).map_err(LoadError::InvalidValue)?;
            if parsed.profiles.len() >= MAX_CONFIG_PROFILES
                && !parsed.profiles.contains_key(profile.as_str())
            {
                return Err(LoadError::TooManyProfiles);
            }
            parsed
                .profiles
                .entry(profile.as_str().to_owned())
                .or_default();
            current_profile = Some(profile.as_str().to_owned());
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(LoadError::ConfigSyntax);
        };
        let key = key.trim();
        let value = value.trim();
        if value.is_empty() {
            return Err(LoadError::ConfigSyntax);
        }
        let scope_key = format!("{}:{key}", current_profile.as_deref().unwrap_or("<root>"));
        if !seen_keys.insert(scope_key) {
            return Err(LoadError::ConfigDuplicateKey);
        }

        match current_profile.as_deref() {
            None if key == "default_profile" => {
                ProfileName::parse(value).map_err(LoadError::InvalidValue)?;
                parsed.default_profile = Some(value.to_owned());
            }
            None => return Err(LoadError::ConfigUnknownKey),
            Some(profile) => {
                let values = parsed
                    .profiles
                    .get_mut(profile)
                    .expect("profile section was inserted");
                match key {
                    "server_url" => values.server_url = Some(value.to_owned()),
                    "output_format" => values.output_format = Some(value.to_owned()),
                    "token_source" => values.token_source = Some(value.to_owned()),
                    _ => return Err(LoadError::ConfigUnknownKey),
                }
            }
        }
    }

    Ok(parsed)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoadError {
    ConfigFileUnavailable,
    ConfigFileTooLarge,
    ConfigFileInvalidEncoding,
    ConfigLineTooLong,
    ConfigSyntax,
    ConfigUnknownSection,
    ConfigUnknownKey,
    ConfigDuplicateKey,
    TooManyProfiles,
    InvalidValue(ConfigError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ConfigFileUnavailable => "configuration file is unavailable",
            Self::ConfigFileTooLarge => "configuration file exceeds the safe size limit",
            Self::ConfigFileInvalidEncoding => "configuration file is not valid UTF-8",
            Self::ConfigLineTooLong => "configuration file contains an overlong line",
            Self::ConfigSyntax => "configuration file syntax is invalid",
            Self::ConfigUnknownSection => "configuration file contains an unsupported section",
            Self::ConfigUnknownKey => "configuration file contains an unsupported key",
            Self::ConfigDuplicateKey => "configuration file contains a duplicate key",
            Self::TooManyProfiles => "configuration file contains too many profiles",
            Self::InvalidValue(error) => return write!(formatter, "configuration error: {error}"),
        })
    }
}

impl std::error::Error for LoadError {}

#[derive(Clone, Eq, PartialEq)]
pub struct SecretToken(String);

impl SecretToken {
    pub(crate) fn parse(value: &str) -> Result<Self, TokenLoadError> {
        let value = value.trim_end_matches(|character| character == '\r' || character == '\n');
        if value.is_empty() {
            return Err(TokenLoadError::EmptyToken);
        }
        if value.len() > MAX_TOKEN_BYTES {
            return Err(TokenLoadError::TokenTooLarge);
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b'"' && byte != b'\\')
        {
            return Err(TokenLoadError::InvalidTokenEncoding);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretToken(<redacted>)")
    }
}

pub trait BearerTokenProvider {
    fn bearer_token(&self) -> Result<Option<SecretToken>, TokenLoadError>;
}

pub struct ProcessTokenProvider {
    source: TokenSource,
    cached: OnceLock<Result<Option<SecretToken>, TokenLoadError>>,
}

impl ProcessTokenProvider {
    #[must_use]
    pub fn new(source: TokenSource) -> Self {
        Self {
            source,
            cached: OnceLock::new(),
        }
    }

    fn load(&self) -> Result<Option<SecretToken>, TokenLoadError> {
        match &self.source {
            TokenSource::None => Ok(None),
            TokenSource::EnvVar(name) => {
                let value = std::env::var(name.as_str())
                    .map_err(|_| TokenLoadError::TokenSourceUnavailable)?;
                SecretToken::parse(&value).map(Some)
            }
            TokenSource::File(path) => {
                let bytes = read_bounded_bytes(Path::new(path.as_str()), MAX_TOKEN_BYTES)?;
                let value = String::from_utf8(bytes)
                    .map_err(|_| TokenLoadError::InvalidTokenEncoding)?;
                SecretToken::parse(&value).map(Some)
            }
            TokenSource::Stdin => {
                let mut input = Vec::new();
                io::stdin()
                    .lock()
                    .take((MAX_TOKEN_BYTES + 1) as u64)
                    .read_to_end(&mut input)
                    .map_err(|_| TokenLoadError::TokenSourceUnavailable)?;
                if input.len() > MAX_TOKEN_BYTES {
                    return Err(TokenLoadError::TokenTooLarge);
                }
                let value = String::from_utf8(input)
                    .map_err(|_| TokenLoadError::InvalidTokenEncoding)?;
                SecretToken::parse(&value).map(Some)
            }
            TokenSource::OsSecret(_) => Err(TokenLoadError::OsSecretUnavailable),
        }
    }
}

impl BearerTokenProvider for ProcessTokenProvider {
    fn bearer_token(&self) -> Result<Option<SecretToken>, TokenLoadError> {
        self.cached.get_or_init(|| self.load()).clone()
    }
}

fn read_bounded_bytes(path: &Path, max_bytes: usize) -> Result<Vec<u8>, TokenLoadError> {
    let mut file = fs::File::open(path).map_err(|_| TokenLoadError::TokenSourceUnavailable)?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take((max_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| TokenLoadError::TokenSourceUnavailable)?;
    if bytes.len() > max_bytes {
        return Err(TokenLoadError::TokenTooLarge);
    }
    Ok(bytes)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenLoadError {
    TokenSourceUnavailable,
    OsSecretUnavailable,
    EmptyToken,
    TokenTooLarge,
    InvalidTokenEncoding,
}

impl fmt::Display for TokenLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TokenSourceUnavailable => "configured token source is unavailable",
            Self::OsSecretUnavailable => {
                "configured OS secret source is unavailable in this CLI build"
            }
            Self::EmptyToken => "configured token source returned an empty token",
            Self::TokenTooLarge => "configured token exceeds the safe size limit",
            Self::InvalidTokenEncoding => "configured token contains unsupported characters",
        })
    }
}

impl std::error::Error for TokenLoadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeSources {
        environment: BTreeMap<String, String>,
        files: BTreeMap<PathBuf, String>,
    }

    impl SourceProvider for FakeSources {
        fn environment(&self, name: &str) -> Option<String> {
            self.environment.get(name).cloned()
        }

        fn read_file(&self, path: &Path, max_bytes: usize) -> Result<Option<String>, LoadError> {
            let value = self.files.get(path).cloned();
            if value.as_ref().is_some_and(|content| content.len() > max_bytes) {
                return Err(LoadError::ConfigFileTooLarge);
            }
            Ok(value)
        }
    }

    #[test]
    fn global_options_are_removed_before_command_parsing() {
        let parsed = split_global_options([
            "haze-sync",
            "--profile=ops",
            "--server-url",
            "http://127.0.0.1:8080",
            "status",
        ])
        .unwrap();
        assert_eq!(parsed.command_args, ["haze-sync", "status"]);
        assert_eq!(parsed.overrides.profile.as_deref(), Some("ops"));
        assert_eq!(
            parsed.overrides.server_url.as_deref(),
            Some("http://127.0.0.1:8080")
        );
    }

    #[test]
    fn cli_environment_file_and_default_precedence_is_deterministic() {
        let path = PathBuf::from("config.test");
        let mut sources = FakeSources::default();
        sources.environment.insert(
            SERVER_URL_ENV.to_owned(),
            "http://environment.test".to_owned(),
        );
        sources.environment.insert(
            OUTPUT_FORMAT_ENV.to_owned(),
            "json".to_owned(),
        );
        sources.files.insert(
            path.clone(),
            "default_profile = ops\n[profile.ops]\nserver_url = http://file.test\noutput_format = human\ntoken_source = env:FILE_TOKEN\n"
                .to_owned(),
        );
        let overrides = ConfigOverrides {
            config_path: Some(path),
            server_url: Some("http://cli.test".to_owned()),
            ..ConfigOverrides::default()
        };

        let config = load_config(&overrides, &sources).unwrap();
        assert_eq!(config.profile.as_str(), "ops");
        assert_eq!(config.server_url.unwrap().as_str(), "http://cli.test");
        assert_eq!(config.output_format, OutputFormat::Json);
        assert!(matches!(config.token_source, TokenSource::EnvVar(_)));
    }

    #[test]
    fn selected_profile_controls_file_values() {
        let path = PathBuf::from("config.test");
        let mut sources = FakeSources::default();
        sources.files.insert(
            path.clone(),
            "default_profile = one\n[profile.one]\nserver_url = http://one.test\n[profile.two]\nserver_url = http://two.test\ntoken_source = none\n"
                .to_owned(),
        );
        let overrides = ConfigOverrides {
            config_path: Some(path),
            profile: Some("two".to_owned()),
            ..ConfigOverrides::default()
        };

        let config = load_config(&overrides, &sources).unwrap();
        assert_eq!(config.profile.as_str(), "two");
        assert_eq!(config.server_url.unwrap().as_str(), "http://two.test");
    }

    #[test]
    fn malformed_duplicate_and_oversized_config_is_rejected_safely() {
        assert_eq!(
            parse_config_file("[other]\nserver_url=http://example.test\n").unwrap_err(),
            LoadError::ConfigUnknownSection
        );
        assert_eq!(
            parse_config_file(
                "[profile.default]\nserver_url=http://one.test\nserver_url=http://two.test\n"
            )
            .unwrap_err(),
            LoadError::ConfigDuplicateKey
        );
        let long = format!("[profile.default]\nserver_url={}\n", "x".repeat(MAX_CONFIG_LINE_BYTES));
        assert_eq!(
            parse_config_file(&long).unwrap_err(),
            LoadError::ConfigLineTooLong
        );
    }

    #[test]
    fn token_debug_and_errors_never_echo_secret_values() {
        let token = SecretToken::parse("redacted-test-token").unwrap();
        assert_eq!(format!("{token:?}"), "SecretToken(<redacted>)");
        let invalid = SecretToken::parse("redacted token").unwrap_err().to_string();
        assert!(!invalid.contains("redacted"));
    }
}
