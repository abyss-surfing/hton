//! Literal unit observations. No patterns, paths, jobs, pager or mutations.
use crate::{
    Budget, Cancellation, Error, Result,
    process::{Runner, Tool},
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitName(String);
impl UnitName {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let base = value
            .strip_suffix(".service")
            .or_else(|| value.strip_suffix(".timer"))
            .ok_or(Error::InvalidInput)?;
        if value.len() > 128
            || base.is_empty()
            || !base.as_bytes()[0].is_ascii_alphanumeric()
            || !base
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-@".contains(&b))
            || base.contains("..")
            || base.ends_with('@')
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone)]
pub struct UnitFacts {
    pub unit: UnitName,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub unit_file_state: String,
    pub main_pid: Option<u32>,
    pub restart: Option<String>,
    pub structural_fingerprint: String,
}
#[derive(Clone)]
pub struct SystemdObserver {
    runner: Arc<Runner>,
}
impl SystemdObserver {
    pub(crate) fn new(runner: Arc<Runner>) -> Self {
        Self { runner }
    }
    pub fn inspect(
        &self,
        expected: &UnitName,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<UnitFacts> {
        let properties = if expected.as_str().ends_with(".service") {
            "Id,LoadState,ActiveState,SubState,UnitFileState,MainPID,Restart,FragmentPath,DropInPaths,ExecStart,ExecStop,User,Group,Type"
        } else {
            "Id,LoadState,ActiveState,SubState,UnitFileState,FragmentPath,DropInPaths,Triggers"
        };
        let bytes = self.runner.run(
            Tool::Systemctl,
            &[
                "--system",
                "--no-pager",
                "--no-ask-password",
                "show",
                "--property",
                properties,
                "--",
                expected.as_str(),
            ],
            budget,
            cancellation,
        )?;
        let text = std::str::from_utf8(&bytes).map_err(|_| Error::InvalidResponse)?;
        let mut values = BTreeMap::new();
        for line in text.lines() {
            let (key, value) = line.split_once('=').ok_or(Error::InvalidResponse)?;
            if !properties.split(',').any(|property| property == key)
                || values.insert(key, value).is_some()
            {
                return Err(Error::InvalidResponse);
            }
        }
        if values.len() != properties.split(',').count() {
            return Err(Error::InvalidResponse);
        }
        let get = |key| values.get(key).copied().ok_or(Error::InvalidResponse);
        if get("Id")? != expected.as_str() {
            return Err(Error::IdentityMismatch);
        }
        let token = |key| -> Result<String> {
            let value = get(key)?;
            if value.len() > 128 || !value.bytes().all(|b| b.is_ascii_lowercase() || b == b'-') {
                return Err(Error::InvalidResponse);
            }
            Ok(value.to_owned())
        };
        if !get("DropInPaths")?.is_empty() {
            return Err(Error::InvalidResponse);
        }
        let fragment = get("FragmentPath")?;
        if !(fragment.starts_with("/etc/systemd/system/")
            || fragment.starts_with("/usr/lib/systemd/system/"))
        {
            return Err(Error::InvalidResponse);
        }
        let path = std::path::Path::new(fragment);
        let parent = path.parent().ok_or(Error::InvalidResponse)?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(Error::InvalidResponse)?;
        let bytes = hton_storage::SecureDir::open(parent)
            .and_then(|dir| dir.read(name, 1024 * 1024))
            .map_err(|_| Error::InvalidResponse)?
            .ok_or(Error::InvalidResponse)?;
        let mut structural_values = BTreeMap::new();
        for (key, value) in &values {
            if !matches!(
                *key,
                "ActiveState" | "SubState" | "UnitFileState" | "MainPID"
            ) {
                structural_values.insert(*key, *value);
            }
        }
        let fingerprint = {
            use sha2::{Digest, Sha256};
            let mut hash = Sha256::new();
            hash.update(
                serde_json::to_vec(&structural_values).map_err(|_| Error::InvalidResponse)?,
            );
            hash.update([0]);
            hash.update(&bytes);
            format!("sha256:{hash:x}", hash = hash.finalize())
        };
        let facts = UnitFacts {
            unit: expected.clone(),
            structural_fingerprint: fingerprint,
            load_state: token("LoadState")?,
            active_state: token("ActiveState")?,
            sub_state: token("SubState")?,
            unit_file_state: token("UnitFileState")?,
            main_pid: values
                .get("MainPID")
                .map(|value| value.parse().map_err(|_| Error::InvalidResponse))
                .transpose()?,
            restart: if values.contains_key("Restart") {
                Some(token("Restart")?)
            } else {
                None
            },
        };
        budget.check(cancellation)?;
        Ok(facts)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum UnitAction {
    Start,
    Stop,
    Restart,
    Enable,
    Disable,
}
#[derive(Clone)]
pub struct SystemdControl {
    runner: Arc<Runner>,
}
impl SystemdControl {
    pub(crate) fn new(runner: Arc<Runner>) -> Self {
        Self { runner }
    }
    /// The caller validates template bytes and exact link destinations before
    /// invoking this fixed installer capability. No arbitrary unit path enters it.
    pub fn link_platform_units(&self, budget: Budget, cancel: &dyn Cancellation) -> Result<()> {
        self.runner.run(
            Tool::Systemctl,
            &[
                "--system",
                "--no-pager",
                "--no-ask-password",
                "link",
                "--",
                "/opt/hton/work/systemd/hton-data.service",
                "/opt/hton/work/systemd/hton-guardian.service",
                "/opt/hton/work/systemd/hton-acme.service",
                "/opt/hton/work/systemd/hton-acme.timer",
                "/opt/hton/work/systemd/hton-native-log@.service",
            ],
            budget,
            cancel,
        )?;
        self.runner
            .run(
                Tool::Systemctl,
                &[
                    "--system",
                    "--no-pager",
                    "--no-ask-password",
                    "daemon-reload",
                ],
                budget,
                cancel,
            )
            .map(|_| ())
    }
    /// Fixed platform maintenance only; the caller must prove whole-server
    /// stopped state and exact installed unit/template ownership beforehand.
    pub fn stop_platform_units(&self, budget: Budget, cancel: &dyn Cancellation) -> Result<()> {
        for name in [
            "hton-acme.timer",
            "hton-acme.service",
            "hton-data.service",
            "hton-guardian.service",
        ] {
            let state = self.runner.run(
                Tool::Systemctl,
                &[
                    "--system",
                    "--no-pager",
                    "--no-ask-password",
                    "show",
                    "--property=LoadState",
                    "--value",
                    "--",
                    name,
                ],
                budget,
                cancel,
            )?;
            match std::str::from_utf8(&state)
                .map_err(|_| Error::InvalidResponse)?
                .trim()
            {
                "not-found" => continue,
                "loaded" | "masked" => {}
                _ => return Err(Error::InvalidResponse),
            }
            if name != "hton-acme.service" {
                self.apply(&UnitName::new(name)?, UnitAction::Disable, budget, cancel)?;
            }
            self.apply(&UnitName::new(name)?, UnitAction::Stop, budget, cancel)?;
        }
        Ok(())
    }
    pub fn reload_manager(&self, budget: Budget, cancel: &dyn Cancellation) -> Result<()> {
        self.runner
            .run(
                Tool::Systemctl,
                &[
                    "--system",
                    "--no-pager",
                    "--no-ask-password",
                    "daemon-reload",
                ],
                budget,
                cancel,
            )
            .map(|_| ())
    }
    /// Missing loaded state is explicit absence, not an observation failure.
    /// The caller still owns exact unit identity and lifecycle authority.
    pub fn stop_if_loaded(
        &self,
        unit: &UnitName,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<bool> {
        let bytes = self.runner.run(
            Tool::Systemctl,
            &[
                "--system",
                "--no-pager",
                "--no-ask-password",
                "show",
                "--property=LoadState",
                "--value",
                "--",
                unit.as_str(),
            ],
            budget,
            cancel,
        )?;
        match std::str::from_utf8(&bytes)
            .map_err(|_| Error::InvalidResponse)?
            .trim()
        {
            "not-found" => Ok(false),
            "loaded" | "masked" => {
                self.apply(unit, UnitAction::Stop, budget, cancel)?;
                Ok(true)
            }
            _ => Err(Error::InvalidResponse),
        }
    }
    pub fn apply(
        &self,
        unit: &UnitName,
        action: UnitAction,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        let verb = match action {
            UnitAction::Start => "start",
            UnitAction::Stop => "stop",
            UnitAction::Restart => "restart",
            UnitAction::Enable => "enable",
            UnitAction::Disable => "disable",
        };
        self.runner
            .run(
                Tool::Systemctl,
                &[
                    "--system",
                    "--no-pager",
                    "--no-ask-password",
                    verb,
                    "--",
                    unit.as_str(),
                ],
                budget,
                cancel,
            )
            .map(|_| ())
    }
}

impl std::fmt::Debug for UnitFacts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnitFacts")
            .field("unit", &self.unit)
            .field("launch", &"[protected]")
            .finish()
    }
}
