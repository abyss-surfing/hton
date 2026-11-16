//! CAS identity repository and current-accepted credential evidence.
use crate::{
    certificates::CertificateName,
    config::digest,
    exports::*,
    host::HostApplication,
    identities::*,
    journal::{ProtectedOriginal, deadline, ensure_mutating, failure},
    secrets::{ConsumerGrant, SecretKind},
};
use hton_contracts::{
    config::{ContentRevision, MembershipState, Ownership},
    operations::*,
};
use hton_runtime_contract as wire;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    schema_version: u32,
    revision: u64,
    entries: BTreeMap<String, IdentityBinding>,
    previous: BTreeMap<String, IdentityBinding>,
    history: Vec<IdentityBinding>,
}
pub struct FileIdentityInventory {
    app: HostApplication,
}
impl FileIdentityInventory {
    pub fn validate_archive_record(bytes: &[u8], ctx: &OperationContext) -> Result<()> {
        Self::parse(Some(bytes), ctx).map(|_| ())
    }

    pub fn open(app: &HostApplication) -> Self {
        Self { app: app.clone() }
    }
    fn key(identity: &ObjectId, profile: &ObjectId) -> String {
        digest(&serde_json::to_vec(&(identity, profile)).expect("validated identifiers serialize"))
            .as_str()[7..]
            .to_owned()
    }
    fn directory(&self, ctx: &OperationContext) -> Result<hton_storage::SecureDir> {
        self.app.work.child("identities").map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "identity inventory unavailable",
            )
        })
    }
    fn parse(bytes: Option<&[u8]>, ctx: &OperationContext) -> Result<Record> {
        let record: Record = match bytes {
            Some(bytes) => serde_json::from_slice(bytes).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "identity inventory corrupt",
                )
            })?,
            None => Record {
                schema_version: 1,
                ..Record::default()
            },
        };
        if record.schema_version != 1
            || record.entries.len() > 1024
            || record.previous.len() > 1024
            || record.history.len() > 4096
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "identity inventory schema/bounds invalid",
            ));
        }
        for (key, binding) in record.entries.iter().chain(&record.previous) {
            if *key != Self::key(&binding.identity_id, &binding.profile_id)
                || binding.service_id.as_str() != "xray"
                || binding.scope.as_str() != "module.vless"
                || (binding.state == BindingState::Pending && binding.accepted_bundle.is_some())
            {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "identity binding invariant invalid",
                ));
            }
        }
        Ok(record)
    }
    fn read(&self, ctx: &OperationContext) -> Result<Record> {
        let bytes = self
            .directory(ctx)?
            .read("bindings.json", 4 * 1024 * 1024)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "identity inventory read failed",
                )
            })?;
        Self::parse(bytes.as_deref(), ctx)
    }
    pub fn bindings(&self, ctx: &OperationContext) -> Result<Vec<IdentityBinding>> {
        Ok(self
            .read(ctx)?
            .entries
            .into_values()
            .filter(|binding| binding.state != BindingState::Revoked)
            .collect())
    }
    pub fn revision(&self, ctx: &OperationContext) -> Result<ContentRevision> {
        let record = self.read(ctx)?;
        let bytes = serde_json::to_vec(&record).map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationConflict,
                "identity revision encoding failed",
            )
        })?;
        Ok(crate::config::content_revision(
            ObjectId::new("state.identities").expect("fixed ID"),
            &bytes,
        ))
    }
    fn mutate(
        &self,
        ctx: &OperationContext,
        operation: impl FnOnce(&mut Record) -> Result<()>,
    ) -> Result<()> {
        ensure_mutating(ctx, self.app.control.as_ref())?;
        let directory = self.directory(ctx)?;
        let locked = directory
            .lock(deadline(ctx, self.app.control.as_ref())?)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "identity inventory lock unavailable",
                )
            })?;
        let bytes = locked.read("bindings.json", 4 * 1024 * 1024).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "identity baseline unavailable",
            )
        })?;
        let mut record = Self::parse(bytes.as_deref(), ctx)?;
        operation(&mut record)?;
        record.revision = record.revision.checked_add(1).ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "identity revision overflow",
            )
        })?;
        let next = serde_json::to_vec(&record).map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationConflict,
                "identity inventory encoding failed",
            )
        })?;
        Self::parse(Some(&next), ctx)?;
        locked
            .compare_exchange("bindings.json", bytes.as_deref(), &next)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "identity inventory durability uncertain",
                )
            })
    }
    fn native_accepted(&self, bundle: &BundleId, ctx: &OperationContext) -> Result<bool> {
        let result = crate::host::runtime::execute(&self.app, wire::ExecutorAction::Observe, ctx)?;
        Ok(result.status == wire::ResultStatus::Succeeded
            && result.run_intent == wire::RunIntent::Running
            && result
                .accepted
                .as_ref()
                .is_some_and(|accepted| accepted.as_str() == bundle.as_str())
            && result.errors.as_slice().is_empty()
            && result.survivors.as_slice().is_empty())
    }
    fn bundle(
        &self,
        bundle: &BundleId,
        ctx: &OperationContext,
    ) -> Result<(wire::CandidateBundle, hton_storage::SecureDir)> {
        let directory = self
            .app
            .work
            .child("bundles")
            .and_then(|dir| dir.child(bundle.as_str()))
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "accepted bundle unavailable"))?;
        let bytes = directory
            .read("manifest.json", wire::MAX_WIRE_BYTES)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "accepted manifest unavailable",
                )
            })?
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "accepted manifest missing",
                )
            })?;
        let manifest: wire::CandidateBundle = wire::decode(&bytes).map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "accepted manifest invalid",
            )
        })?;
        if manifest.bundle_id.as_str() != bundle.as_str()
            || wire::candidate_digest(&manifest).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "accepted manifest digest invalid",
                )
            })? != manifest.manifest_digest
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "accepted manifest fingerprint mismatch",
            ));
        }
        Ok((manifest, directory))
    }
    fn xray(
        &self,
        bundle: &BundleId,
        ctx: &OperationContext,
    ) -> Result<(wire::CandidateBundle, Value)> {
        let (manifest, directory) = self.bundle(bundle, ctx)?;
        let artifact = manifest
            .artifacts
            .as_slice()
            .iter()
            .find(|artifact| {
                artifact.service_id.as_str() == "xray"
                    && artifact.path.as_str().ends_with("config.json")
            })
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "accepted Xray config artifact missing",
                )
            })?;
        let mut dir = directory;
        let parts: Vec<_> = artifact.path.as_str().split('/').collect();
        for part in &parts[..parts.len() - 1] {
            dir = dir.child(part).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "accepted artifact directory unavailable",
                )
            })?;
        }
        let bytes = dir
            .read(parts[parts.len() - 1], 16 * 1024 * 1024)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "accepted Xray artifact unavailable",
                )
            })?
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "accepted Xray artifact missing",
                )
            })?;
        if wire::hash(&bytes) != artifact.digest {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "accepted Xray artifact fingerprint mismatch",
            ));
        }
        let config = serde_json::from_slice(&bytes).map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "accepted Xray config invalid",
            )
        })?;
        Ok((manifest, config))
    }
    fn has_uuid(config: &Value, uuid: &str) -> bool {
        config
            .get("inbounds")
            .and_then(Value::as_array)
            .is_some_and(|inbounds| {
                inbounds.iter().any(|inbound| {
                    inbound["protocol"] == "vless"
                        && inbound["settings"]["clients"]
                            .as_array()
                            .is_some_and(|clients| {
                                clients
                                    .iter()
                                    .any(|client| client["id"].as_str() == Some(uuid))
                            })
                })
            })
    }
    fn uuid(&self, binding: &IdentityBinding, ctx: &OperationContext) -> Result<String> {
        let material = self.app.secrets.resolve(
            &binding.secret,
            &ConsumerGrant {
                consumer: binding.service_id.clone(),
                scope: binding.scope.clone(),
            },
            ctx,
        )?;
        let value = std::str::from_utf8(material.expose()).map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "identity credential encoding invalid",
            )
        })?;
        Ok(value.to_owned())
    }
    pub fn revocation_approval_id(identity: &ObjectId, profile: &ObjectId) -> ApprovalId {
        ApprovalId::new(format!("identity-revoke.{}", Self::key(identity, profile)))
            .expect("bounded digest ID")
    }
}
impl IdentityInventory for FileIdentityInventory {
    fn run_state(&self, ctx: &OperationContext) -> Result<(RunIntent, Option<BundleId>)> {
        let state = self.app.runtime(ctx)?;
        Ok((
            if state.run_intent == wire::RunIntent::Running {
                RunIntent::Running
            } else {
                RunIntent::Stopped
            },
            state
                .accepted
                .map(|id| BundleId::new(id.as_str()).expect("wire ID")),
        ))
    }
    fn authorize_issue(&self, _: &IdentityIssueRequest, ctx: &OperationContext) -> Result<bool> {
        Ok(ctx.actor.source == ActorSource::Local
            && ctx
                .actor
                .granted_scopes
                .contains(&ScopeId::new("identities.manage").expect("fixed scope")))
    }
    fn authorize_revoke(
        &self,
        request: &IdentityRevokeRequest,
        ctx: &OperationContext,
    ) -> Result<bool> {
        Ok(self.authorize_issue(
            &IdentityIssueRequest {
                identity_id: request.identity_id.clone(),
                profile_id: request.profile_id.clone(),
                replace: false,
                approval_id: None,
            },
            ctx,
        )? && request.approval_id
            == Self::revocation_approval_id(&request.identity_id, &request.profile_id))
    }
    fn profile(&self, id: &ObjectId, ctx: &OperationContext) -> Result<QualifiedIdentityProfile> {
        if id.as_str() != "vless" {
            return Err(failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "only R1 VLESS identity profile is supported",
            ));
        }
        let registry = self.app.registry(ctx)?;
        let entry = registry
            .entries
            .get(&ModuleId::new("vless").expect("fixed module"));
        let read = self.app.desired(ctx)?;
        let desired = read.snapshot.ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "desired configuration is invalid",
            )
        })?;
        Ok(QualifiedIdentityProfile {
            profile_id: id.clone(),
            service_id: ServiceId::new("xray").expect("fixed service"),
            scope: ScopeId::new("module.vless").expect("fixed scope"),
            enabled: desired
                .features
                .get(&ModuleId::new("vless").expect("fixed module"))
                .is_some_and(|feature| feature.enabled),
            installed: entry.is_some_and(|entry| entry.membership == MembershipState::Installed),
            qualified: true,
            supports_independent_credentials: true,
            credential_kind: SecretKind::UuidV4,
            externally_owned: entry.is_some_and(|entry| entry.ownership != Ownership::Owned),
        })
    }
    fn binding(
        &self,
        identity: &ObjectId,
        profile: &ObjectId,
        ctx: &OperationContext,
    ) -> Result<Option<IdentityBinding>> {
        Ok(self
            .read(ctx)?
            .entries
            .get(&Self::key(identity, profile))
            .cloned())
    }
    fn by_transaction(
        &self,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<Option<IdentityBinding>> {
        let record = self.read(ctx)?;
        Ok(record
            .entries
            .into_values()
            .chain(record.history)
            .find(|binding| binding.transaction_id == *transaction))
    }
    fn originals(
        &self,
        _: &ObjectId,
        _: &ObjectId,
        ctx: &OperationContext,
    ) -> Result<Vec<ProtectedOriginal>> {
        let Some(bytes) = self
            .directory(ctx)?
            .read("bindings.json", 4 * 1024 * 1024)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "identity original unavailable",
                )
            })?
        else {
            return Ok(vec![]);
        };
        Ok(vec![ProtectedOriginal {
            artifact_id: ArtifactId::new("state.identities").expect("fixed ID"),
            digest: digest(&bytes),
            was_present: true,
            bytes,
        }])
    }
    fn save_pending(
        &self,
        binding: &IdentityBinding,
        previous: Option<&IdentityBinding>,
        ctx: &OperationContext,
    ) -> Result<()> {
        if binding.state != BindingState::Pending || binding.accepted_bundle.is_some() {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "only pending identity metadata can be saved",
            ));
        }
        self.mutate(ctx, |record| {
            let key = Self::key(&binding.identity_id, &binding.profile_id);
            if record.entries.get(&key) != previous {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "identity baseline changed",
                ));
            }
            if let Some(old) = previous {
                if old.state == BindingState::Accepted {
                    record.previous.insert(key.clone(), old.clone());
                }
            }
            record.entries.insert(key, binding.clone());
            Ok(())
        })
    }
    fn plan_issue(&self, binding: &IdentityBinding, ctx: &OperationContext) -> Result<EffectPlan> {
        crate::host::effects::prepare_identity(&self.app, binding, true, ctx)
    }
    fn plan_revoke(&self, binding: &IdentityBinding, ctx: &OperationContext) -> Result<EffectPlan> {
        crate::host::effects::prepare_identity(&self.app, binding, false, ctx)
    }
    fn accepted_verified(
        &self,
        binding: &IdentityBinding,
        bundle: &BundleId,
        ctx: &OperationContext,
    ) -> Result<bool> {
        if !self.native_accepted(bundle, ctx)? {
            return Ok(false);
        }
        let (manifest, config) = self.xray(bundle, ctx)?;
        let uuid = self.uuid(binding, ctx)?;
        Ok(manifest.secrets.as_slice().iter().any(|secret| {
            secret.secret_id.as_str() == binding.secret.secret_id.as_str()
                && secret.version.as_str() == binding.secret.version.as_str()
                && secret.service_id.as_str() == binding.service_id.as_str()
        }) && Self::has_uuid(&config, &uuid))
    }
    fn revocation_verified(
        &self,
        binding: &IdentityBinding,
        ctx: &OperationContext,
    ) -> Result<bool> {
        let state = self.app.runtime(ctx)?;
        let Some(accepted) = &state.accepted else {
            return Ok(false);
        };
        let accepted = BundleId::new(accepted.as_str()).expect("wire ID");
        if !self.native_accepted(&accepted, ctx)? {
            return Ok(false);
        }
        let uuid = self.uuid(binding, ctx)?;
        let (manifest, config) = self.xray(&accepted, ctx)?;
        if Self::has_uuid(&config, &uuid)
            || manifest.fences.credential_revision < state.fences.credential_revision
        {
            return Ok(false);
        }
        if let Some(previous) = state.previous_compatible {
            let (manifest, config) =
                self.xray(&BundleId::new(previous.as_str()).expect("wire ID"), ctx)?;
            if manifest.fences.credential_revision >= state.fences.credential_revision
                && Self::has_uuid(&config, &uuid)
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn finalize(
        &self,
        binding: &IdentityBinding,
        state: BindingState,
        bundle: Option<&BundleId>,
        ctx: &OperationContext,
    ) -> Result<IdentityBinding> {
        if state == BindingState::Pending || bundle.is_none() {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "identity finalization needs accepted evidence",
            ));
        }
        let mut updated = binding.clone();
        updated.state = state;
        updated.accepted_bundle = bundle.cloned();
        self.mutate(ctx, |record| {
            let key = Self::key(&binding.identity_id, &binding.profile_id);
            if record.entries.get(&key) != Some(binding) {
                if record.entries.get(&key) == Some(&updated) {
                    return Ok(());
                }
                return Err(failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "identity finalization baseline changed",
                ));
            }
            if let Some(previous) = record.previous.remove(&key) {
                record.history.push(previous);
            }
            record.entries.insert(key, updated.clone());
            Ok(())
        })?;
        Ok(updated)
    }
}
impl ExportInventory for FileIdentityInventory {
    fn accepted(
        &self,
        identity: &ObjectId,
        profile: &ObjectId,
        ctx: &OperationContext,
    ) -> Result<AcceptedCredentialBinding> {
        let record = self.read(ctx)?;
        let key = Self::key(identity, profile);
        let state = self.app.runtime(ctx)?;
        let bundle = state
            .accepted
            .as_ref()
            .map(|id| BundleId::new(id.as_str()).expect("wire ID"))
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::RuntimeStopped,
                    "no accepted credential bundle",
                )
            })?;
        let current = record
            .entries
            .get(&key)
            .ok_or_else(|| failure(ctx, ErrorCode::ModuleUnknown, "identity not found"))?;
        let binding = if current.state == BindingState::Pending {
            record.previous.get(&key).unwrap_or(current)
        } else {
            current
        };
        if binding.state != BindingState::Accepted
            || !self.accepted_verified(binding, &bundle, ctx)?
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "identity has no verified current accepted credential",
            ));
        }
        let (manifest, config) = self.xray(&bundle, ctx)?;
        let endpoint = manifest
            .claims
            .as_slice()
            .iter()
            .filter_map(|claim| match claim {
                wire::Claim::Certificate {
                    service, hostname, ..
                } if service.as_str() == "caddy" => CertificateName::new(hostname.as_str()).ok(),
                _ => None,
            })
            .next()
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "accepted HTTPS endpoint absent",
                )
            })?;
        let port = manifest
            .claims
            .as_slice()
            .iter()
            .find_map(|claim| match claim {
                wire::Claim::Listener { service, port, .. } if service.as_str() == "haproxy" => {
                    Some(*port)
                }
                _ => None,
            })
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "accepted public listener absent",
                )
            })?;
        let inbound = config["inbounds"]
            .as_array()
            .and_then(|inbounds| {
                inbounds
                    .iter()
                    .find(|inbound| inbound["protocol"] == "vless")
            })
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "accepted VLESS inbound absent",
                )
            })?;
        let network = inbound["streamSettings"]["network"]
            .as_str()
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "accepted transport absent",
                )
            })?;
        let settings = &inbound["streamSettings"][if network == "ws" {
            "wsSettings"
        } else {
            "xhttpSettings"
        }];
        let path = settings["path"]
            .as_str()
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "accepted transport path absent",
                )
            })?
            .to_owned();
        let transport = match network {
            "ws" => VlessTransport::WebSocket {
                path,
                host: endpoint.clone(),
            },
            "xhttp" => VlessTransport::XHttp {
                path,
                host: endpoint.clone(),
                mode: match settings["mode"].as_str().unwrap_or("auto") {
                    "auto" => XHttpMode::Auto,
                    "packet-up" => XHttpMode::PacketUp,
                    "stream-up" => XHttpMode::StreamUp,
                    "stream-one" => XHttpMode::StreamOne,
                    _ => {
                        return Err(failure(
                            ctx,
                            ErrorCode::ModuleUnsupported,
                            "accepted XHTTP mode unsupported",
                        ));
                    }
                },
            },
            _ => {
                return Err(failure(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "accepted VLESS transport unsupported",
                ));
            }
        };
        Ok(AcceptedCredentialBinding {
            identity_id: identity.clone(),
            profile_id: profile.clone(),
            accepted_bundle: bundle,
            service_id: binding.service_id.clone(),
            secret: binding.secret.clone(),
            scope: binding.scope.clone(),
            enabled: true,
            revoked: false,
            credential_fence: state.fences.credential_revision,
            endpoint: endpoint.clone(),
            port,
            server_name: endpoint,
            transport,
            display_name: identity.as_str().to_owned(),
        })
    }
    fn current_accepted(&self, ctx: &OperationContext) -> Result<(BundleId, u64)> {
        let state = self.app.runtime(ctx)?;
        if state.run_intent != wire::RunIntent::Running {
            return Err(failure(
                ctx,
                ErrorCode::RuntimeStopped,
                "accepted profile is not running",
            ));
        }
        let bundle = state
            .accepted
            .ok_or_else(|| failure(ctx, ErrorCode::ModuleIncomplete, "accepted bundle absent"))?;
        Ok((
            BundleId::new(bundle.as_str()).expect("wire ID"),
            state.fences.credential_revision,
        ))
    }
    fn disclosure_allowed(
        &self,
        _: &AcceptedCredentialBinding,
        ctx: &OperationContext,
    ) -> Result<bool> {
        Ok(ctx.actor.source == ActorSource::Local
            && ctx
                .actor
                .granted_scopes
                .contains(&ScopeId::new("identities.disclose").expect("fixed scope")))
    }
    fn client_qualified(
        &self,
        client: &ObjectId,
        _: &VlessTransport,
        format: &ExportFormat,
        _: &OperationContext,
    ) -> Result<bool> {
        Ok(client.as_str() == "native" && *format == ExportFormat::XrayJson)
    }
}
