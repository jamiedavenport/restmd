//! The resolution [`Context`]: the variable sources and the injectable
//! clock / id generator the builtins draw on.
//!
//! Build one with [`Context::builder`]. The four `{{var}}` lookup tiers (spec
//! §4.5) are stored separately so precedence is explicit at lookup time. The
//! OS-environment snapshot backs both tier 3 (`RESTMD_VAR_*`) and the
//! `env(NAME)` builtin.

use std::collections::BTreeMap;
use std::time::SystemTime;

use crate::model::ConfigValue;

/// Prefix for tier-3 environment-variable lookups.
pub(crate) const ENV_VAR_PREFIX: &str = "RESTMD_VAR_";

/// Everything resolution needs: the variable tiers plus the clock/id sources
/// for non-deterministic builtins.
pub struct Context {
    /// Tier 1: values captured from earlier requests in the same run.
    pub(crate) captures: BTreeMap<String, String>,
    /// Tier 2: CLI `--var key=value` flags.
    pub(crate) vars: BTreeMap<String, String>,
    /// OS environment snapshot. Backs tier 3 (keys under `RESTMD_VAR_`) and the
    /// `env(NAME)` builtin (raw key).
    pub(crate) os_env: BTreeMap<String, String>,
    /// Tier 4: the selected frontmatter environment block, stringified.
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) clock: Box<dyn Clock>,
    pub(crate) idgen: Box<dyn IdGen>,
}

impl Context {
    /// Start building a context. All sources default to empty; the clock and id
    /// generator default to the real system implementations.
    pub fn builder() -> ContextBuilder {
        ContextBuilder::default()
    }

    /// Tier 1–4 variable lookup, first match wins (spec §4.5).
    pub(crate) fn lookup(&self, name: &str) -> Option<&str> {
        self.captures
            .get(name)
            .or_else(|| self.vars.get(name))
            .or_else(|| self.os_env.get(&format!("{ENV_VAR_PREFIX}{name}")))
            .or_else(|| self.environment.get(name))
            .map(String::as_str)
    }

    /// Raw OS-environment lookup for the `env(NAME)` builtin.
    pub(crate) fn os_env(&self, name: &str) -> Option<&str> {
        self.os_env.get(name).map(String::as_str)
    }
}

/// Builder for [`Context`]. Every setter is additive/overriding; call
/// [`build`](ContextBuilder::build) to finish.
pub struct ContextBuilder {
    captures: BTreeMap<String, String>,
    vars: BTreeMap<String, String>,
    os_env: BTreeMap<String, String>,
    environment: BTreeMap<String, String>,
    clock: Box<dyn Clock>,
    idgen: Box<dyn IdGen>,
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self {
            captures: BTreeMap::new(),
            vars: BTreeMap::new(),
            os_env: BTreeMap::new(),
            environment: BTreeMap::new(),
            clock: Box::new(SystemClock),
            idgen: Box::new(RandomIdGen),
        }
    }
}

impl ContextBuilder {
    /// Tier 1: captured values.
    pub fn captures(mut self, m: BTreeMap<String, String>) -> Self {
        self.captures = m;
        self
    }

    /// Tier 2: CLI `--var` values.
    pub fn vars(mut self, m: BTreeMap<String, String>) -> Self {
        self.vars = m;
        self
    }

    /// Set the OS-environment snapshot explicitly (useful in tests).
    pub fn os_env(mut self, m: BTreeMap<String, String>) -> Self {
        self.os_env = m;
        self
    }

    /// Snapshot the real process environment into the context.
    pub fn os_env_from_process(mut self) -> Self {
        self.os_env = std::env::vars().collect();
        self
    }

    /// Tier 4: the selected frontmatter environment block. [`ConfigValue`]s are
    /// stringified via their [`Display`](std::fmt::Display) impl.
    pub fn environment(mut self, m: &BTreeMap<String, ConfigValue>) -> Self {
        self.environment = m.iter().map(|(k, v)| (k.clone(), v.to_string())).collect();
        self
    }

    /// Override the clock (for deterministic `now()` / `timestamp()`).
    pub fn clock(mut self, clock: impl Clock + 'static) -> Self {
        self.clock = Box::new(clock);
        self
    }

    /// Override the id generator (for deterministic `uuid()`).
    pub fn idgen(mut self, idgen: impl IdGen + 'static) -> Self {
        self.idgen = Box::new(idgen);
        self
    }

    pub fn build(self) -> Context {
        Context {
            captures: self.captures,
            vars: self.vars,
            os_env: self.os_env,
            environment: self.environment,
            clock: self.clock,
            idgen: self.idgen,
        }
    }
}

/// Source of the current time for the `now()` / `timestamp()` builtins.
/// Injectable so those builtins can be tested deterministically.
pub trait Clock {
    fn now(&self) -> SystemTime;
}

/// Source of UUIDs for the `uuid()` builtin. Injectable for deterministic tests.
pub trait IdGen {
    fn uuid(&self) -> String;
}

/// Production clock: the real system time.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

/// Production id generator: random UUID v4.
pub struct RandomIdGen;

impl IdGen for RandomIdGen {
    fn uuid(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}
