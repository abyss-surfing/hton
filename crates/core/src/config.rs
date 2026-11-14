//! Bounded, read-only desired configuration decoding and deterministic normalization.
use crate::catalog::{ModuleCatalogue, ResolutionEvidence};
use hton_contracts::{config::*, operations::*};
use hton_storage::SecureDir;
use serde::de::{DeserializeSeed, Error as DeError, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::Path,
};
use yaml_rust2::scanner::{Scanner, TokenType};

const ROOT_SCHEMA: &str = include_str!("../../../platform/schemas/hton-v1.schema.json");
const MODULE_SCHEMA: &str = include_str!("../../../platform/schemas/module-v1.schema.json");

#[derive(Clone, Copy, Debug)]
pub struct LoadLimits {
    pub file_bytes: usize,
    pub total_bytes: usize,
    pub files: usize,
    pub depth: usize,
    pub nodes: usize,
}
impl Default for LoadLimits {
    fn default() -> Self {
        Self {
            file_bytes: 1_048_576,
            total_bytes: 8_388_608,
            files: 128,
            depth: 32,
            nodes: 65_536,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceError {
    Unavailable,
    Unsafe,
    TooLarge,
    Conflict,
}
impl From<hton_storage::Error> for SourceError {
    fn from(value: hton_storage::Error) -> Self {
        match value {
            hton_storage::Error::TooLarge => Self::TooLarge,
            hton_storage::Error::Conflict => Self::Conflict,
            hton_storage::Error::UnsafePath
            | hton_storage::Error::InvalidName
            | hton_storage::Error::NotRegular => Self::Unsafe,
            _ => Self::Unavailable,
        }
    }
}
/// All names are fixed inventory components; implementations must refuse symlinks.
pub trait ConfigSource {
    fn list_modules(&self, limit: usize) -> std::result::Result<Vec<String>, SourceError>;
    fn list_docker(&self, limit: usize) -> std::result::Result<Vec<String>, SourceError>;
    fn read(
        &self,
        components: &[&str],
        limit: usize,
    ) -> std::result::Result<Option<Vec<u8>>, SourceError>;
}
pub struct FsConfigSource {
    root: SecureDir,
}
impl FsConfigSource {
    pub fn open(path: &Path) -> std::result::Result<Self, SourceError> {
        Ok(Self {
            root: SecureDir::open(path)?,
        })
    }
    fn list(&self, directory: &str, limit: usize) -> std::result::Result<Vec<String>, SourceError> {
        match self.root.child(directory) {
            Ok(dir) => Ok(dir.list(limit)?),
            Err(hton_storage::Error::NotFound) => Ok(Vec::new()),
            Err(error) => Err(error.into()),
        }
    }
}
impl ConfigSource for FsConfigSource {
    fn list_modules(&self, limit: usize) -> std::result::Result<Vec<String>, SourceError> {
        self.list("modules", limit)
    }
    fn list_docker(&self, limit: usize) -> std::result::Result<Vec<String>, SourceError> {
        self.list("docker", limit)
    }
    fn read(
        &self,
        components: &[&str],
        limit: usize,
    ) -> std::result::Result<Option<Vec<u8>>, SourceError> {
        let Some((name, parents)) = components.split_last() else {
            return Err(SourceError::Unsafe);
        };
        let mut directory = self.root.try_clone()?;
        for parent in parents {
            directory = match directory.child(parent) {
                Ok(child) => child,
                Err(hton_storage::Error::NotFound) => return Ok(None),
                Err(error) => return Err(error.into()),
            };
        }
        Ok(directory.read(name, limit)?)
    }
}

/// SHA-256 binds exact bytes and logical object identity.
pub fn content_revision(object_id: ObjectId, bytes: &[u8]) -> ContentRevision {
    let mut hash = Sha256::new();
    hash.update(object_id.as_str().as_bytes());
    hash.update([0]);
    hash.update(bytes);
    ContentRevision {
        object_id,
        digest: digest_bytes(&hash.finalize()),
    }
}
pub fn digest(bytes: &[u8]) -> Sha256Digest {
    digest_bytes(&Sha256::digest(bytes))
}

/// Build the canonical first-install desired state over a caller-verified virtual
/// absence snapshot. This performs no IO and never invents module settings bytes.
pub fn initial_desired(
    instance_id: &InstanceId,
    selected_modules: &BTreeSet<ModuleId>,
    absent_read_set: &ReadSet,
    catalogue: &ModuleCatalogue,
) -> std::result::Result<DesiredReadResult, CheckIssue> {
    let root_id = config_id("root");
    if absent_read_set.coherence != ReadCoherence::Coherent
        || !absent_read_set.objects.is_empty()
        || !absent_read_set.absent_objects.contains(&root_id)
        || !absent_read_set.secret_versions.is_empty()
        || absent_read_set
            .absent_objects
            .iter()
            .any(|id| absent_read_set.objects.contains_key(id))
    {
        return Err(issue(
            ErrorCode::ConfigReadConflict,
            Some(root_id),
            None,
            "Initial configuration requires a coherent expected-absent inventory",
        ));
    }

    for object_id in absent_read_set
        .absent_objects
        .iter()
        .filter(|id| id.as_str() != "root")
    {
        let Some(module) = object_id.as_str().strip_prefix("module.") else {
            return Err(issue(
                ErrorCode::ConfigSemanticConflict,
                Some(object_id.clone()),
                None,
                "Initial absence inventory contains an unknown configuration object",
            ));
        };
        let known = ModuleId::new(module.to_owned())
            .ok()
            .is_some_and(|module_id| catalogue.describe(&module_id).is_some());
        if !known {
            return Err(issue(
                ErrorCode::ConfigSemanticConflict,
                Some(object_id.clone()),
                None,
                "Initial absence inventory contains an unknown module settings object",
            ));
        }
    }

    let schema: Value = serde_json::from_str(ROOT_SCHEMA).expect("embedded root schema");
    let allowed_features = schema["properties"]["features"]["properties"]
        .as_object()
        .ok_or_else(|| {
            issue(
                ErrorCode::ConfigSyntax,
                Some(root_id.clone()),
                None,
                "Embedded root feature schema is invalid",
            )
        })?;
    let mut features = BTreeMap::new();
    for (name, _) in allowed_features {
        let Ok(module_id) = ModuleId::new(name.clone()) else {
            continue;
        };
        if catalogue.describe(&module_id).is_some() {
            features.insert(
                module_id.clone(),
                FeatureSelection {
                    enabled: selected_modules.contains(&module_id),
                },
            );
        }
    }
    for module_id in selected_modules {
        let settings_id =
            ObjectId::new(format!("module.{}", module_id.as_str())).map_err(|_| {
                issue(
                    ErrorCode::ConfigSemanticConflict,
                    None,
                    Some(module_id.clone()),
                    "Selected module identity is invalid",
                )
            })?;
        if !features.contains_key(module_id)
            || !absent_read_set.absent_objects.contains(&settings_id)
        {
            return Err(issue(
                ErrorCode::ConfigSemanticConflict,
                Some(settings_id),
                Some(module_id.clone()),
                "Selected module must be known and its settings document proven absent",
            ));
        }
    }

    let snapshot = DesiredSnapshot {
        platform: PlatformSettings {
            schema_version: FormatVersion(1),
            settings: initial_root_settings(instance_id),
        },
        features,
        module_settings: BTreeMap::new(),
        editable_docker: BTreeMap::new(),
        read_set: absent_read_set.clone(),
    };
    let root_document = root_document(&snapshot)?;
    if !schema_valid(ROOT_SCHEMA, &root_document) {
        return Err(issue(
            ErrorCode::ConfigSyntax,
            Some(root_id),
            None,
            "Canonical initial configuration does not match the root schema",
        ));
    }

    Ok(DesiredReadResult {
        documents: Vec::new(),
        snapshot: Some(snapshot),
        errors: Vec::new(),
        read_set: absent_read_set.clone(),
    })
}

/// Canonical root document projection shared by virtual preflight and bootstrap.
pub fn root_document(snapshot: &DesiredSnapshot) -> std::result::Result<Value, CheckIssue> {
    if snapshot.platform.schema_version != FormatVersion(1) {
        return Err(issue(
            ErrorCode::ConfigUnsupportedVersion,
            Some(config_id("root")),
            None,
            "Unsupported root format version for serialization",
        ));
    }
    let mut root = Map::from_iter(snapshot.platform.settings.clone());
    root.insert(
        "schemaVersion".into(),
        Value::from(snapshot.platform.schema_version.0),
    );
    let mut features = Map::new();
    for (module_id, selection) in &snapshot.features {
        features.insert(
            module_id.as_str().into(),
            serde_json::json!({"enabled": selection.enabled}),
        );
    }
    root.insert("features".into(), Value::Object(features));
    let value = Value::Object(root);
    if !schema_valid(ROOT_SCHEMA, &value) {
        return Err(issue(
            ErrorCode::ConfigSyntax,
            Some(config_id("root")),
            None,
            "Desired root snapshot does not match the supported schema",
        ));
    }
    Ok(value)
}

fn initial_root_settings(instance_id: &InstanceId) -> BTreeMap<String, Value> {
    serde_json::from_value(serde_json::json!({
        "instance": {"name": instance_id.as_str()},
        "installation": {"systemd": true, "pathShim": false},
        "logging": {
            "level": "info",
            "directory": "/var/logs/hton",
            "maxBytes": 67108864u64,
            "retentionDays": 7
        },
        "core": {
            "edge": {
                "listen": {
                    "transport": "tcp",
                    "port": 4443,
                    "scope": "host-loopback",
                    "addressFamilies": ["ipv4"]
                },
                "domains": []
            }
        },
        "onboarding": {
            "defaultConnection": "gateway",
            "preferredTargets": []
        }
    }))
    .expect("canonical initial root fields are a JSON object")
}
fn digest_bytes(bytes: &[u8]) -> Sha256Digest {
    let value = format!(
        "sha256:{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    Sha256Digest::new(value).expect("SHA256 output has fixed canonical encoding")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_matches_known_sha256() {
        // SHA-256("abc") — RFC 6234 / FIPS 180-4 known answer.
        let expect = "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(digest(b"abc").as_str(), expect);
    }

    #[test]
    fn digest_is_deterministic_and_input_sensitive() {
        assert_eq!(digest(b"abc"), digest(b"abc"));
        assert_ne!(digest(b"abc"), digest(b"abd"));
        assert_ne!(digest(b""), digest(b"a"));
    }

    #[test]
    fn content_revision_binds_object_identity() {
        let id_a = ObjectId::new("object-a").unwrap();
        let id_b = ObjectId::new("object-b").unwrap();
        assert_eq!(
            content_revision(id_a.clone(), b"payload").digest,
            content_revision(id_a.clone(), b"payload").digest,
        );
        assert_ne!(
            content_revision(id_a.clone(), b"payload").digest,
            content_revision(id_b, b"payload").digest,
        );
        assert_ne!(
            content_revision(id_a.clone(), b"payload").digest,
            content_revision(id_a, b"other").digest,
        );
    }
}
pub fn issue(
    code: ErrorCode,
    object: Option<ObjectId>,
    module: Option<ModuleId>,
    message: &'static str,
) -> CheckIssue {
    CheckIssue {
        code,
        severity: Severity::Error,
        module_id: module,
        object_id: object,
        path: None,
        message: message.into(),
        repair_hint: None,
    }
}
fn config_id(value: &str) -> ObjectId {
    ObjectId::new(value).expect("trusted static object ID")
}
fn empty_read_set() -> ReadSet {
    ReadSet {
        objects: BTreeMap::new(),
        absent_objects: BTreeSet::new(),
        secret_versions: BTreeMap::new(),
        coherence: ReadCoherence::Unverified,
    }
}

pub fn load(
    source: &dyn ConfigSource,
    supported_versions: &BTreeSet<FormatVersion>,
) -> DesiredReadResult {
    load_with_limits(source, supported_versions, LoadLimits::default())
}
pub fn load_with_limits(
    source: &dyn ConfigSource,
    supported_versions: &BTreeSet<FormatVersion>,
    limits: LoadLimits,
) -> DesiredReadResult {
    let mut result = DesiredReadResult {
        documents: Vec::new(),
        snapshot: None,
        errors: Vec::new(),
        read_set: empty_read_set(),
    };
    if limits.file_bytes == 0
        || limits.total_bytes < limits.file_bytes
        || limits.files == 0
        || limits.depth == 0
        || limits.depth > 64
        || limits.nodes == 0
    {
        result.errors.push(issue(
            ErrorCode::ConfigSemanticConflict,
            None,
            None,
            "Invalid trusted configuration read limits",
        ));
        return result;
    }
    let observed_inventory = match inventory(source, limits.files) {
        Ok(inventory) => inventory,
        Err(_) => {
            result.errors.push(issue(
                ErrorCode::IoUnavailable,
                None,
                None,
                "Configuration inventory is unavailable or unsafe",
            ));
            return result;
        }
    };
    let mut total = 0usize;
    let mut raw = BTreeMap::<ObjectId, (Vec<String>, Vec<u8>)>::new();
    let mut decoded = BTreeMap::new();
    for (id, components) in &observed_inventory {
        let parts: Vec<_> = components.iter().map(String::as_str).collect();
        let bytes = match source.read(&parts, limits.file_bytes) {
            Ok(Some(bytes)) if bytes.len() <= limits.file_bytes => bytes,
            _ => {
                result.errors.push(issue(
                    ErrorCode::IoUnavailable,
                    Some(id.clone()),
                    None,
                    "Configuration document is missing, unsafe or exceeds its read budget",
                ));
                continue;
            }
        };
        total = total.saturating_add(bytes.len());
        if total > limits.total_bytes {
            result.errors.push(issue(
                ErrorCode::ConfigSyntax,
                Some(id.clone()),
                None,
                "Configuration tree exceeds its read budget",
            ));
            break;
        }
        let revision = content_revision(id.clone(), &bytes);
        let mut diagnostics = Vec::new();
        let value = match decode_yaml(&bytes, limits) {
            Ok(value) => Some(value),
            Err(()) => {
                diagnostics.push(issue(ErrorCode::ConfigSyntax, Some(id.clone()), None, "Invalid or unsafe YAML: use unique string keys without tags, anchors or aliases within the size and depth limits"));
                None
            }
        };
        let version = value
            .as_ref()
            .and_then(|v| v.get("schemaVersion"))
            .and_then(Value::as_u64)
            .map(FormatVersion);
        if !id.as_str().starts_with("docker.") {
            match version {
                Some(version) if supported_versions.contains(&version) => {}
                _ => diagnostics.push(issue(
                    ErrorCode::ConfigUnsupportedVersion,
                    Some(id.clone()),
                    None,
                    "Configuration format version is absent or unsupported",
                )),
            }
            if let Some(value) = &value {
                let schema = if id.as_str() == "root" {
                    ROOT_SCHEMA
                } else {
                    MODULE_SCHEMA
                };
                if !schema_valid(schema, value) {
                    diagnostics.push(issue(ErrorCode::ConfigSyntax, Some(id.clone()), None, "Configuration does not match its known schema; inspect required fields, field types and unknown keys"));
                }
                if let Some(module) = id.as_str().strip_prefix("module.") {
                    if value.get("moduleId").and_then(Value::as_str) != Some(module) {
                        diagnostics.push(issue(
                            ErrorCode::ConfigSemanticConflict,
                            Some(id.clone()),
                            ModuleId::new(module).ok(),
                            "Module document identity differs from its inventory identity",
                        ));
                    }
                }
            }
        }
        result.read_set.objects.insert(
            id.clone(),
            ReadDependency {
                revision: revision.clone(),
                format_version: version.unwrap_or(FormatVersion(1)),
            },
        );
        result.errors.extend(diagnostics.clone());
        result.documents.push(ConfigDocument {
            object_id: id.clone(),
            location: KnownLocation {
                inventory_id: config_id("config"),
                components: components
                    .iter()
                    .map(|v| ObjectId::new(v.clone()).expect("inventory validated component"))
                    .collect(),
            },
            source_bytes: bytes.clone(),
            revision,
            schema_version: version,
            diagnostics,
        });
        raw.insert(id.clone(), (components.clone(), bytes));
        if let Some(value) = value {
            decoded.insert(id.clone(), value);
        }
    }
    let reread = inventory(source, limits.files);
    let mut stable = reread
        .as_ref()
        .is_ok_and(|second| second == &observed_inventory);
    for (components, expected) in raw.values() {
        let parts: Vec<_> = components.iter().map(String::as_str).collect();
        if !source
            .read(&parts, limits.file_bytes)
            .is_ok_and(|actual| actual.as_ref() == Some(expected))
        {
            stable = false;
        }
    }
    result.read_set.coherence = if stable {
        ReadCoherence::Coherent
    } else {
        ReadCoherence::Conflict
    };
    if !stable {
        result.errors.push(issue(
            ErrorCode::ConfigReadConflict,
            None,
            None,
            "Configuration changed during the read; retry after the editor finishes",
        ));
    }
    if result.errors.is_empty() {
        if let Some(root) = decoded.remove(&config_id("root")) {
            let mut root = root.as_object().cloned().unwrap_or_default();
            root.remove("schemaVersion");
            let feature_value = root
                .remove("features")
                .unwrap_or_else(|| Value::Object(Map::new()));
            let features = feature_value
                .as_object()
                .into_iter()
                .flatten()
                .filter_map(|(id, value)| {
                    Some((
                        ModuleId::new(id.clone()).ok()?,
                        FeatureSelection {
                            enabled: value.get("enabled")?.as_bool()?,
                        },
                    ))
                })
                .collect();
            let mut module_settings = BTreeMap::new();
            let mut editable_docker = BTreeMap::new();
            for (id, value) in decoded {
                if let Some(module) = id.as_str().strip_prefix("module.") {
                    if let Ok(module_id) = ModuleId::new(module) {
                        let settings = value
                            .get("settings")
                            .and_then(Value::as_object)
                            .cloned()
                            .unwrap_or_default()
                            .into_iter()
                            .collect();
                        module_settings.insert(
                            module_id,
                            ModuleSettings {
                                schema_version: FormatVersion(
                                    value
                                        .get("schemaVersion")
                                        .and_then(Value::as_u64)
                                        .unwrap_or(1),
                                ),
                                settings,
                            },
                        );
                    }
                } else if let Some(service) = id.as_str().strip_prefix("docker.") {
                    if let (Ok(service_id), Some(document)) = (
                        ServiceId::new(service),
                        result.documents.iter().find(|d| d.object_id == id),
                    ) {
                        editable_docker.insert(service_id, document.clone());
                    }
                }
            }
            result.snapshot = Some(DesiredSnapshot {
                platform: PlatformSettings {
                    schema_version: FormatVersion(1),
                    settings: root.into_iter().collect(),
                },
                features,
                module_settings,
                editable_docker,
                read_set: result.read_set.clone(),
            });
        }
    }
    result
}
fn inventory(
    source: &dyn ConfigSource,
    limit: usize,
) -> std::result::Result<BTreeMap<ObjectId, Vec<String>>, SourceError> {
    let mut entries = BTreeMap::from([(config_id("root"), vec!["hton.yaml".into()])]);
    for (directory, prefix, names) in [
        ("modules", "module", source.list_modules(limit)?),
        ("docker", "docker", source.list_docker(limit)?),
    ] {
        if names.len() > limit {
            return Err(SourceError::TooLarge);
        }
        for name in names {
            // Preserved originals are not live input documents.
            if name.ends_with(".deprecated.yml") || name.ends_with(".deprecated.yaml") {
                continue;
            }
            let Some(stem) = name
                .strip_suffix(".yaml")
                .or_else(|| name.strip_suffix(".yml"))
            else {
                continue;
            };
            ObjectId::new(name.clone()).map_err(|_| SourceError::Unsafe)?;
            ModuleId::new(stem).map_err(|_| SourceError::Unsafe)?;
            let id = ObjectId::new(format!("{prefix}.{stem}")).map_err(|_| SourceError::Unsafe)?;
            if entries.insert(id, vec![directory.into(), name]).is_some() {
                return Err(SourceError::Conflict);
            }
            if entries.len() > limit {
                return Err(SourceError::TooLarge);
            }
        }
    }
    Ok(entries)
}
fn schema_valid(schema: &str, value: &Value) -> bool {
    let Ok(schema) = serde_json::from_str::<Value>(schema) else {
        return false;
    };
    let Ok(validator) = jsonschema::validator_for(&schema) else {
        return false;
    };
    validator.is_valid(value)
}

/// No source text is included in the returned diagnostic error.
pub fn decode_yaml(bytes: &[u8], limits: LoadLimits) -> std::result::Result<Value, ()> {
    if bytes.len() > limits.file_bytes {
        return Err(());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| ())?;
    let mut scanner = Scanner::new(text.chars());
    let mut depth = 0usize;
    let mut count = 0usize;
    for token in scanner.by_ref() {
        count += 1;
        if count > limits.nodes.saturating_mul(4) {
            return Err(());
        }
        match token.1 {
            TokenType::Alias(_)
            | TokenType::Anchor(_)
            | TokenType::Tag(_, _)
            | TokenType::TagDirective(_, _)
            | TokenType::VersionDirective(_, _) => return Err(()),
            TokenType::BlockSequenceStart
            | TokenType::BlockMappingStart
            | TokenType::FlowSequenceStart
            | TokenType::FlowMappingStart => {
                depth += 1;
                if depth > limits.depth {
                    return Err(());
                }
            }
            TokenType::BlockEnd | TokenType::FlowSequenceEnd | TokenType::FlowMappingEnd => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    if scanner.get_error().is_some() {
        return Err(());
    }
    let mut documents = serde_yaml::Deserializer::from_str(text);
    let document = documents.next().ok_or(())?;
    let remaining = Cell::new(limits.nodes);
    let value = ValueSeed {
        depth: limits.depth,
        remaining: &remaining,
    }
    .deserialize(document)
    .map_err(|_| ())?;
    if documents.next().is_some() {
        return Err(());
    }
    Ok(value)
}
struct ValueSeed<'a> {
    depth: usize,
    remaining: &'a Cell<usize>,
}
impl<'de> DeserializeSeed<'de> for ValueSeed<'_> {
    type Value = Value;
    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Value, D::Error> {
        if self.depth == 0 || self.remaining.get() == 0 {
            return Err(D::Error::custom("YAML budget exceeded"));
        }
        self.remaining.set(self.remaining.get() - 1);
        deserializer.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for ValueSeed<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("bounded data-only YAML")
    }
    fn visit_bool<E: DeError>(self, v: bool) -> std::result::Result<Value, E> {
        Ok(Value::Bool(v))
    }
    fn visit_i64<E: DeError>(self, v: i64) -> std::result::Result<Value, E> {
        Ok(v.into())
    }
    fn visit_u64<E: DeError>(self, v: u64) -> std::result::Result<Value, E> {
        Ok(v.into())
    }
    fn visit_f64<E: DeError>(self, v: f64) -> std::result::Result<Value, E> {
        serde_json::Number::from_f64(v)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite YAML number"))
    }
    fn visit_str<E: DeError>(self, v: &str) -> std::result::Result<Value, E> {
        Ok(Value::String(v.into()))
    }
    fn visit_string<E: DeError>(self, v: String) -> std::result::Result<Value, E> {
        Ok(Value::String(v))
    }
    fn visit_unit<E: DeError>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_none<E: DeError>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> std::result::Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(ValueSeed {
            depth: self.depth - 1,
            remaining: self.remaining,
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut mapping: A) -> std::result::Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = mapping.next_key_seed(StringKey)? {
            if values.contains_key(&key) || key == "<<" {
                return Err(A::Error::custom("duplicate or merge YAML key"));
            }
            let value = mapping.next_value_seed(ValueSeed {
                depth: self.depth - 1,
                remaining: self.remaining,
            })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}
struct StringKey;
impl<'de> DeserializeSeed<'de> for StringKey {
    type Value = String;
    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<String, D::Error> {
        deserializer.deserialize_any(self)
    }
}
impl Visitor<'_> for StringKey {
    type Value = String;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a string mapping key")
    }
    fn visit_str<E: DeError>(self, value: &str) -> std::result::Result<String, E> {
        Ok(value.into())
    }
    fn visit_string<E: DeError>(self, value: String) -> std::result::Result<String, E> {
        Ok(value)
    }
}

pub trait SecretAvailability {
    /// Return supplied version evidence, never secret material or a generated value.
    fn version(&self, canonical_id: &SecretId) -> Option<SecretVersion>;
}
pub struct ValidationPorts<'a> {
    pub release: ReleasePhase,
    pub resolution: &'a ResolutionEvidence,
    pub module_checks: BTreeMap<ModuleId, &'a dyn ModuleCheck>,
    pub secret_checks: &'a dyn SecretAvailability,
}
pub fn normalize(snapshot: &DesiredSnapshot, catalogue: &ModuleCatalogue) -> CheckReport {
    let mut desired = snapshot.clone();
    let mut issues = Vec::new();
    let mut provenance = Vec::new();
    // Typed callers have the same schema obligations as filesystem callers.
    let mut root: Map<String, Value> = snapshot.platform.settings.clone().into_iter().collect();
    root.insert(
        "schemaVersion".into(),
        Value::from(snapshot.platform.schema_version.0),
    );
    root.insert(
        "features".into(),
        serde_json::to_value(&snapshot.features).unwrap_or(Value::Null),
    );
    if !schema_valid(ROOT_SCHEMA, &Value::Object(root)) {
        issues.push(issue(
            ErrorCode::ConfigSyntax,
            Some(config_id("root")),
            None,
            "Desired platform settings do not match the supported root schema",
        ));
    }
    for (id, settings) in &snapshot.module_settings {
        let value = serde_json::json!({"schemaVersion":settings.schema_version.0,"moduleId":id,"settings":settings.settings});
        if !schema_valid(MODULE_SCHEMA, &value) {
            issues.push(issue(
                ErrorCode::ConfigSyntax,
                None,
                Some(id.clone()),
                "Module settings do not match their known schema",
            ));
        }
    }
    for (section, defaults) in [
        (
            "installation",
            serde_json::json!({"systemd":true,"pathShim":false}),
        ),
        (
            "logging",
            serde_json::json!({"level":"info","directory":"/var/logs/hton","maxBytes":67108864u64,"retentionDays":7}),
        ),
        (
            "onboarding",
            serde_json::json!({"defaultConnection":"gateway","preferredTargets":[]}),
        ),
    ] {
        let entry = desired
            .platform
            .settings
            .entry(section.into())
            .or_insert_with(|| Value::Object(Map::new()));
        if let (Some(settings), Some(defaults)) = (entry.as_object_mut(), defaults.as_object()) {
            for (key, value) in defaults {
                let source = if settings.contains_key(key) {
                    ValueProvenance::Explicit
                } else {
                    settings.insert(key.clone(), value.clone());
                    ValueProvenance::Default
                };
                provenance.push(ProvenanceEntry {
                    object_id: config_id("root"),
                    path: ConfigPath(vec![config_id(section), config_id(key)]),
                    source,
                });
            }
        }
    }
    if let Some(listen) = desired
        .platform
        .settings
        .get_mut("core")
        .and_then(|core| core.get_mut("edge"))
        .and_then(|edge| edge.get_mut("listen"))
        .and_then(Value::as_object_mut)
    {
        let source = if listen.contains_key("scope") {
            ValueProvenance::Explicit
        } else {
            listen.insert("scope".into(), Value::String("public".into()));
            ValueProvenance::Default
        };
        provenance.push(ProvenanceEntry {
            object_id: config_id("root"),
            path: ConfigPath(vec![
                config_id("core"),
                config_id("edge"),
                config_id("listen"),
                config_id("scope"),
            ]),
            source,
        });
    }

    for id in desired.features.keys() {
        if catalogue.describe(id).is_none() {
            issues.push(issue(
                ErrorCode::ModuleUnknown,
                None,
                Some(id.clone()),
                "Unknown feature identity is retained but cannot activate",
            ));
        }
        if id.as_str() == "core" {
            issues.push(issue(
                ErrorCode::ConfigSemanticConflict,
                None,
                Some(id.clone()),
                "Mandatory core is not a feature toggle",
            ));
        }
    }
    let root_schema: Value = serde_json::from_str(ROOT_SCHEMA).expect("embedded root schema");
    for (id, entry) in catalogue.entries() {
        if root_schema["properties"]["features"]["properties"]
            .get(id.as_str())
            .is_some()
        {
            desired
                .features
                .entry(id.clone())
                .or_insert(FeatureSelection { enabled: false });
        }
        if let Some(settings) = desired.module_settings.get_mut(id) {
            if settings.schema_version != entry.descriptor.config_version {
                issues.push(issue(
                    ErrorCode::ConfigUnsupportedVersion,
                    None,
                    Some(id.clone()),
                    "Module settings version is unsupported",
                ));
                continue;
            }
            for (key, default) in &entry.descriptor.defaults.settings {
                let source = if settings.settings.contains_key(key) {
                    ValueProvenance::Explicit
                } else {
                    settings.settings.insert(key.clone(), default.clone());
                    ValueProvenance::Default
                };
                provenance.push(ProvenanceEntry {
                    object_id: config_id(&format!("module.{id}")),
                    path: ConfigPath(vec![config_id("settings"), config_id(key)]),
                    source,
                });
            }
        }
    }
    let supported_versions = desired
        .read_set
        .objects
        .iter()
        .map(|(id, dependency)| (id.clone(), dependency.format_version))
        .collect();
    let blocking = !issues.is_empty();
    CheckReport {
        issues,
        evidence: Vec::new(),
        blocking,
        normalized_snapshot: if blocking {
            None
        } else {
            Some(NormalizedSnapshot {
                desired,
                provenance,
                supported_versions,
                resolved_references: Vec::new(),
            })
        },
    }
}
pub fn validate(
    snapshot: &DesiredSnapshot,
    catalogue: &ModuleCatalogue,
    registry: &RegistrySnapshot,
    approvals: &[ApprovalEnvelope],
    ports: &ValidationPorts<'_>,
) -> CheckReport {
    let mut report = normalize(snapshot, catalogue);
    let Some(mut normalized) = report.normalized_snapshot.take() else {
        return report;
    };
    if snapshot.read_set.coherence != ReadCoherence::Coherent {
        report.issues.push(issue(
            ErrorCode::ConfigReadConflict,
            None,
            None,
            "Configuration read set is not coherent",
        ));
    }
    let graph = catalogue.resolve(&normalized, ports.release, ports.resolution);
    report.issues.extend(graph.conflicts.clone());
    let observations = &ports.resolution.observations;
    for id in &graph.enabled_modules {
        if id.as_str() == "core" {
            continue;
        }
        match registry.entries.get(id) {
            Some(entry)
                if entry.membership == MembershipState::Installed
                    && entry.setup_state == SetupState::Complete => {}
            _ => report.issues.push(issue(
                ErrorCode::ModuleIncomplete,
                None,
                Some(id.clone()),
                "Enabled module is not completely installed or associated",
            )),
        }
        let Some(settings) = normalized.desired.module_settings.get(id) else {
            report.issues.push(issue(
                ErrorCode::ModuleIncomplete,
                None,
                Some(id.clone()),
                "Enabled module settings document is missing",
            ));
            continue;
        };
        match ports.module_checks.get(id) {
            Some(checker) => report
                .issues
                .extend(checker.check(settings, &graph, observations)),
            None => report.issues.push(issue(
                ErrorCode::ModuleUnsupported,
                None,
                Some(id.clone()),
                "Enabled module has no trusted semantic checker",
            )),
        }
        if let Some(entry) = catalogue.describe(id) {
            for privilege in &entry.descriptor.privileges {
                if !approvals.iter().any(|a| {
                    a.privileges.contains(privilege)
                        && a.expires_at
                            .is_none_or(|expiry| expiry.0 > ports.resolution.now.0)
                }) {
                    report.issues.push(issue(
                        ErrorCode::ApprovalRequired,
                        None,
                        Some(id.clone()),
                        "Enabled module requires a locally granted privilege",
                    ));
                }
            }
        }
        let value = Value::Object(settings.settings.clone().into_iter().collect());
        collect_secret_versions(
            &value,
            id,
            ports.secret_checks,
            &mut normalized.desired.read_set,
            &mut report.issues,
        );
    }
    if let Some(core) = normalized
        .desired
        .platform
        .settings
        .get("core")
        .cloned()
        .filter(|core| {
            core["edge"]["domains"]
                .as_array()
                .is_some_and(|domains| !domains.is_empty())
        })
    {
        if core.get("certificates").is_none() {
            report.issues.push(issue(
                ErrorCode::ModuleIncomplete,
                None,
                None,
                "Published HTTPS domains require an explicit native certificate configuration",
            ));
        }
        collect_secret_versions(
            &core,
            &ModuleId::new("core").expect("static ID"),
            ports.secret_checks,
            &mut normalized.desired.read_set,
            &mut report.issues,
        );
    }
    report.blocking = report
        .issues
        .iter()
        .any(|issue| issue.severity == Severity::Error);
    if !report.blocking {
        report.normalized_snapshot = Some(normalized);
    }
    report
}
pub fn canonical_secret_id(reference: &str) -> Option<SecretId> {
    let parts: Vec<_> = reference.split('/').collect();
    if parts.len() < 2
        || parts.iter().any(|part| {
            part.is_empty()
                || !part.as_bytes()[0].is_ascii_lowercase()
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
    {
        return None;
    }
    SecretId::new(parts.join(".")).ok()
}
fn collect_secret_versions(
    value: &Value,
    module: &ModuleId,
    available: &dyn SecretAvailability,
    read_set: &mut ReadSet,
    issues: &mut Vec<CheckIssue>,
) {
    match value {
        Value::Object(values) => {
            if let Some(reference) = values.get("secretRef") {
                match reference.as_str().and_then(canonical_secret_id).and_then(|id| available.version(&id).map(|version| (id,version))) {
                    Some((id,version)) => { read_set.secret_versions.insert(id,version); },
                    None => issues.push(issue(ErrorCode::ModuleIncomplete, None, Some(module.clone()), "Required secret reference is malformed or its protected version is unavailable")),
                }
            }
            for child in values.values() {
                collect_secret_versions(child, module, available, read_set, issues);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_secret_versions(child, module, available, read_set, issues);
            }
        }
        _ => {}
    }
}
