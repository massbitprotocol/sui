use envconfig::Envconfig;
use lazy_static::lazy_static;
use std::{collections::HashSet, env::VarError, fmt, str::FromStr, time::Duration};
lazy_static! {
    pub static ref ENV_VARS: EnvVars = EnvVars::from_env().unwrap();
}
/// Panics if:
/// - The value is not UTF8.
/// - The value cannot be parsed as T..
pub fn env_var<E: std::error::Error + Send + Sync, T: FromStr<Err = E> + Eq>(
    name: &'static str,
    default_value: T,
) -> T {
    let var = match std::env::var(name) {
        Ok(var) => var,
        Err(VarError::NotPresent) => return default_value,
        Err(VarError::NotUnicode(_)) => panic!("environment variable {} is not UTF8", name),
    };

    var.parse::<T>()
        .unwrap_or_else(|e| panic!("failed to parse environment variable {}: {}", name, e))
}

#[derive(Clone)]
#[non_exhaustive]
pub struct EnvVars {
    /// A
    /// [`chrono`](https://docs.rs/chrono/latest/chrono/#formatting-and-parsing)
    /// -like format string for logs.
    ///
    /// Set by the environment variable `SCALAR_LOG_TIME_FORMAT`. The default
    /// value is `%b %d %H:%M:%S%.3f`.
    pub log_time_format: String,
    /// Set by the environment variable `SCALAR_LOG`.
    pub log_levels: Option<String>,
}

impl EnvVars {
    pub fn from_env() -> Result<Self, envconfig::Error> {
        let inner = Inner::init_from_env()?;
        Ok(Self {
            log_time_format: inner.log_time_format,
            log_levels: inner.log_levels,
        })
    }
}

impl Default for EnvVars {
    fn default() -> Self {
        ENV_VARS.clone()
    }
}

// This does not print any values avoid accidentally leaking any sensitive env vars
impl fmt::Debug for EnvVars {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "env vars")
    }
}

#[derive(Clone, Debug, Envconfig)]
struct Inner {
    #[envconfig(from = "SCALAR_LOG_TIME_FORMAT", default = "%b %d %H:%M:%S%.3f")]
    log_time_format: String,
    #[envconfig(from = "SCALAR_LOG")]
    log_levels: Option<String>,
}
