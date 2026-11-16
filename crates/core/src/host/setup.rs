//! Concrete fixed setup providers; coordinator remains the sole sequencing owner.
use super::HostApplication;
use crate::{
    catalog::ModuleCatalogue,
    config,
    journal::{self, Intent, OperationControl, ensure_mutating},
    registry::RegistryPreconditions,
    selection::{self, SelectionWriter},
    setup::{SetupBackend, SetupCoordinator, SetupInventory, SetupProgressEvidence, SetupResult},
};
use hton_contracts::{config::*, operations::*};
use hton_host::{
    Budget as HostBudget, HostObserver,
    docker::{ContainerId, ImageReference, OwnedEngine, container_fingerprints},
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageLock {
    schema_version: u32,
    images: BTreeMap<String, LockedImage>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LockedImage {
    reference: String,
    manifest_digest: ImageDigest,
    #[serde(default)]
    platforms: Vec<LockedPlatform>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LockedPlatform {
    os: String,
    architecture: String,
    manifest_digest: ImageDigest,
}

struct SetupCancellation<'a> {
    control: &'a dyn OperationControl,
    id: &'a CancellationId,
}
impl hton_host::Cancellation for SetupCancellation<'_> {
    fn cancelled(&self) -> bool {
        self.control.is_cancelled(self.id)
    }
}
fn preview_error(module: &ModuleId, code: ErrorCode, message: &'static str) -> Error {
    Error {
        code,
        message: message.into(),
        module_id: Some(module.clone()),
        config_path: None,
        resource_id: None,
        operation_id: OperationId::new("setup.preview").expect("fixed ID"),
        retryability: Retryability::AfterReconcile,
        repair_hint: None,
        retained_state: RetainedState::OriginalsRetained,
    }
}

struct NativeSetup {
    app: HostApplication,
    module: ModuleId,
    images: BTreeMap<String, LockedImage>,
    host: HostObserver,
}
impl NativeSetup {
    fn new(app: &HostApplication, module: ModuleId, ctx: &OperationContext) -> Result<Self> {
        if !matches!(module.as_str(), "core" | "vless") {
            return Err(journal::failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "no R1 owned setup provider",
            ));
        }
        let lock: ImageLock = serde_json::from_str(include_str!(
            "../../../../platform/images.lock.json"
        ))
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "trusted image lock is invalid",
            )
        })?;
        if lock.schema_version != 1 {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigUnsupportedVersion,
                "trusted image lock version is invalid",
            ));
        }
        for name in ["haproxy", "caddy", "xray", "unbound", "acme"] {
            let image = lock.images.get(name).ok_or_else(|| {
                journal::failure(ctx, ErrorCode::ModuleIncomplete, "R1 image pin missing")
            })?;
            if !image.reference.ends_with(image.manifest_digest.as_str())
                || !image.reference.contains('@')
            {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ConfigSyntax,
                    "R1 image reference/digest mismatch",
                ));
            }
            ImageReference::new(image.reference.clone()).map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigSyntax,
                    "invalid R1 immutable image reference",
                )
            })?;
        }
        Ok(Self {
            app: app.clone(),
            module,
            images: lock.images,
            host: HostObserver::new(),
        })
    }
    fn error(&self, ctx: &OperationContext, code: ErrorCode, message: &'static str) -> Error {
        journal::failure(ctx, code, message)
    }
    fn budget(&self, ctx: &OperationContext) -> Result<HostBudget> {
        let remaining = ctx
            .deadline
            .0
            .saturating_sub(self.app.control.now_ms())
            .min(300_000);
        if remaining == 0 || self.app.control.is_cancelled(&ctx.cancellation) {
            return Err(self.error(ctx, ErrorCode::OperationDeadline, "setup budget exhausted"));
        }
        HostBudget::new(
            Instant::now() + Duration::from_millis(remaining),
            2 * 1024 * 1024,
        )
        .map_err(|_| self.error(ctx, ErrorCode::OperationDeadline, "setup budget invalid"))
    }
    fn image(&self, name: &str, ctx: &OperationContext) -> Result<ImageReference> {
        ImageReference::new(
            self.images
                .get(name)
                .ok_or_else(|| {
                    self.error(
                        ctx,
                        ErrorCode::ModuleUnsupported,
                        "image is outside fixed setup recipe",
                    )
                })?
                .reference
                .clone(),
        )
        .map_err(|_| self.error(ctx, ErrorCode::ConfigSyntax, "immutable image pin invalid"))
    }
    fn steps(&self) -> Vec<SetupStep> {
        let mut steps = Vec::new();
        if self.module.as_str() == "core" {
            for name in ["haproxy", "caddy", "xray", "unbound", "acme"] {
                let image = &self.images[name];
                steps.push(self.step(
                    &format!("image.{name}"),
                    SetupStepKind::ObtainImage {
                        service_id: ServiceId::new(name).expect("fixed ID"),
                        digest: image.manifest_digest.clone(),
                    },
                    BTreeSet::new(),
                ));
            }
            for name in ["haproxy", "caddy", "xray", "unbound"] {
                let dependency=steps.iter().find(|step|matches!(&step.kind,SetupStepKind::ObtainImage{service_id,..} if service_id.as_str()==name)).expect("fixed image prerequisite").step_id.clone();
                steps.push(self.step(
                    &format!("resource.{name}"),
                    SetupStepKind::RegisterOwnedResource {
                        resource_id: ResourceId::new(format!("engine.{name}")).expect("fixed ID"),
                    },
                    BTreeSet::from([dependency]),
                ));
            }
        } else {
            steps.push(self.step(
                "settings",
                SetupStepKind::SeedMissingConfig {
                    object_id: ObjectId::new("module.vless").expect("fixed ID"),
                    template_id: ArtifactId::new("template.vless.v1").expect("fixed ID"),
                },
                BTreeSet::new(),
            ));
        }
        let dependencies = steps.iter().map(|step| step.step_id.clone()).collect();
        steps.push(
            self.step(
                "compatibility",
                SetupStepKind::VerifyCompatibility {
                    capability_id: CapabilityId::new(if self.module.as_str() == "core" {
                        "r1.core.cached-launch"
                    } else {
                        "r1.vless.storage"
                    })
                    .expect("fixed ID"),
                },
                dependencies,
            ),
        );
        steps
    }
    fn step(&self, label: &str, kind: SetupStepKind, dependencies: BTreeSet<StepId>) -> SetupStep {
        let hash = config::digest(
            &serde_json::to_vec(&(&self.module, &kind, "local-setup-v1"))
                .expect("fixed setup JSON"),
        );
        SetupStep {step_id:StepId::new(format!("setup.{}.{label}",self.module)).expect("fixed ID"),idempotency_key:RequestId::new(format!("setup.{}",&hash.as_str()[7..])).expect("digest ID"),kind,dependencies,timeout:DurationMillis(120_000),required_approval:None,
            operator_alternative:"Inspect the typed setup result; repair the named local dependency or settings, then repeat install/modules add. No foreign resources are adopted.".into()}
    }
    fn valid(&self, step: &SetupStep, ctx: &OperationContext) -> Result<()> {
        if !self.steps().contains(step) {
            return Err(self.error(
                ctx,
                ErrorCode::OperationDenied,
                "setup step is outside fixed trusted recipe",
            ));
        }
        Ok(())
    }
    fn receipt(&self, step: &SetupStep, status: StepStatus) -> StepReceipt {
        StepReceipt {
            step_id: step.step_id.clone(),
            idempotency_key: step.idempotency_key.clone(),
            status,
            recorded_at: UnixMillis(self.app.control.now_ms()),
            resources: vec![],
            state_fingerprints: BTreeMap::new(),
            retained_artifacts: vec![],
            error: None,
        }
    }
    fn engine(name: &str) -> Option<OwnedEngine> {
        match name {
            "haproxy" => Some(OwnedEngine::Haproxy),
            "caddy" => Some(OwnedEngine::Caddy),
            "xray" => Some(OwnedEngine::Xray),
            "unbound" => Some(OwnedEngine::Unbound),
            _ => None,
        }
    }
    fn module_settings(&self, ctx: &OperationContext) -> Result<Option<ModuleSettings>> {
        let modules = self.app.config.child("modules").map_err(|_| {
            self.error(
                ctx,
                ErrorCode::IoUnavailable,
                "module settings directory unavailable",
            )
        })?;
        let Some(_bytes) = modules
            .read("vless.yaml", config::LoadLimits::default().file_bytes)
            .map_err(|_| {
                self.error(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "module settings unavailable or unsafe",
                )
            })?
        else {
            return Ok(None);
        };
        let desired = self.app.desired(ctx)?;
        if !desired.errors.is_empty() {
            return Err(self.error(
                ctx,
                ErrorCode::ConfigSyntax,
                "configuration tree requires repair; existing settings retained",
            ));
        }
        let settings = desired
            .snapshot
            .and_then(|snapshot| snapshot.module_settings.get(&self.module).cloned())
            .ok_or_else(|| {
                self.error(
                    ctx,
                    ErrorCode::ConfigSyntax,
                    "module settings schema/identity unavailable",
                )
            })?;
        Ok(Some(settings))
    }
    fn registered(&self, name: &str, ctx: &OperationContext) -> Result<Option<ResourceIdentity>> {
        let registry = self.app.registry(ctx)?;
        let resource = ResourceId::new(format!("engine.{name}")).expect("fixed ID");
        Ok(registry
            .entries
            .get(&ModuleId::new("core").expect("fixed ID"))
            .and_then(|entry| entry.resources.iter().find(|r| r.resource_id == resource))
            .cloned())
    }
    fn observed(
        &self,
        name: &str,
        id: &ContainerId,
        ctx: &OperationContext,
    ) -> Result<(ResourceIdentity, ResourceObservation, Sha256Digest)> {
        let cancel = SetupCancellation {
            control: self.app.control.as_ref(),
            id: &ctx.cancellation,
        };
        let facts = self
            .host
            .docker()
            .inspect_container(id, self.budget(ctx)?, &cancel)
            .map_err(|_| {
                self.error(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "exact setup container observation failed",
                )
            })?;
        let fps = container_fingerprints(&facts, None).map_err(|_| {
            self.error(
                ctx,
                ErrorCode::ResourceDrift,
                "native setup fingerprints unavailable",
            )
        })?;
        let identity = ResourceIdentity {
            resource_id: ResourceId::new(format!("engine.{name}")).expect("fixed ID"),
            service_id: ServiceId::new(name).expect("fixed ID"),
            locator: ResourceLocator::DockerContainer {
                container_id: DockerContainerId::new(id.as_str()).map_err(|_| {
                    self.error(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "invalid observed Docker identity",
                    )
                })?,
            },
            fingerprint: Sha256Digest::new(fps.structural).map_err(|_| {
                self.error(ctx, ErrorCode::ResourceDrift, "invalid native fingerprint")
            })?,
        };
        let state = match fps.status {
            "running" => ObservedState::Running,
            "stopped" => ObservedState::Stopped,
            "failed" => ObservedState::Failed,
            _ => ObservedState::Unknown,
        };
        let now = self.app.control.now_ms();
        let observation = ResourceObservation {
            identity: identity.clone(),
            state,
            observed_at: UnixMillis(now),
            valid_until: UnixMillis(now.saturating_add(30_000)),
            restart_policy: Some(match fps.restart_policy {
                "disabled" => RestartPolicy::DockerNo,
                "always" => RestartPolicy::DockerAlways,
                "unless_stopped" => RestartPolicy::DockerUnlessStopped,
                "on_failure" => RestartPolicy::DockerOnFailure {
                    maximum_retry_count: facts.host_config.restart_policy.maximum_retry_count,
                },
                _ => {
                    return Err(self.error(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "unsupported native restart policy",
                    ));
                }
            }),
            evidence: vec![],
        };
        let state_fp = Sha256Digest::new(fps.state).map_err(|_| {
            self.error(
                ctx,
                ErrorCode::ResourceDrift,
                "invalid native state fingerprint",
            )
        })?;
        Ok((identity, observation, state_fp))
    }
    fn prior_intent(&self, step: &SetupStep, ctx: &OperationContext) -> Result<bool> {
        Ok(self.app.journal.unfinished(ctx)?.iter().any(|tx| tx.operation_id!=ctx.operation_id && matches!(&tx.intent,Intent::Setup{plan} if plan.module_id==self.module&&plan.steps.contains(step))))
    }
    fn resource_receipt(
        &self,
        step: &SetupStep,
        name: &str,
        ctx: &OperationContext,
    ) -> Result<StepReceipt> {
        if let Some(recorded) = self.registered(name, ctx)? {
            let ResourceLocator::DockerContainer { container_id } = &recorded.locator else {
                return Err(self.error(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "owned engine locator kind changed",
                ));
            };
            let id = ContainerId::new(container_id.as_str()).map_err(|_| {
                self.error(ctx, ErrorCode::ResourceDrift, "owned engine ID invalid")
            })?;
            let (identity, observation, state_fp) = self.observed(name, &id, ctx)?;
            if identity != recorded
                || matches!(
                    observation.state,
                    ObservedState::Unknown | ObservedState::Failed
                )
            {
                return Err(self.error(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "recorded owned engine drifted",
                ));
            }
            let registry = self.app.registry(ctx)?;
            let complete = registry.entries.get(&self.module).is_some_and(|entry| {
                entry.setup_state == SetupState::Complete
                    && entry.membership == MembershipState::Installed
            });
            if !complete
                && (observation.state != ObservedState::Stopped
                    || observation.restart_policy != Some(RestartPolicy::DockerNo))
            {
                return Err(self.error(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "incomplete setup engine must remain stopped/no-restart",
                ));
            }
            let mut receipt = self.receipt(step, StepStatus::Verified);
            receipt
                .state_fingerprints
                .insert(identity.resource_id.clone(), state_fp);
            receipt.resources.push(identity);
            return Ok(receipt);
        }
        let engine = Self::engine(name).ok_or_else(|| {
            self.error(
                ctx,
                ErrorCode::ModuleUnsupported,
                "engine outside setup recipe",
            )
        })?;
        let cancel = SetupCancellation {
            control: self.app.control.as_ref(),
            id: &ctx.cancellation,
        };
        let image = self.image(name, ctx)?;
        let found = self
            .host
            .docker_control()
            .find_parked(
                self.app.installation.instance_id.as_str(),
                name,
                &image,
                engine,
                self.budget(ctx)?,
                &cancel,
            )
            .map_err(|_| {
                self.error(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "parked native lookup failed; no replay",
                )
            })?;
        let Some(id) = found else {
            return Ok(self.receipt(step, StepStatus::Prepared));
        };
        if !self.prior_intent(step, ctx)? {
            return Err(self.error(
                ctx,
                ErrorCode::ResourceDrift,
                "unexpected preexisting container cannot be adopted by setup",
            ));
        }
        self.fresh_parked_receipt(step, name, &id, ctx)
    }
    fn fresh_parked_receipt(
        &self,
        step: &SetupStep,
        name: &str,
        id: &ContainerId,
        ctx: &OperationContext,
    ) -> Result<StepReceipt> {
        let (identity, observation, state_fp) = self.observed(name, id, ctx)?;
        if observation.state != ObservedState::Stopped
            || observation.restart_policy != Some(RestartPolicy::DockerNo)
        {
            return Err(self.error(
                ctx,
                ErrorCode::ResourceDrift,
                "new owned resource is not verified stopped/no-restart",
            ));
        }
        let mut receipt = self.receipt(step, StepStatus::Verified);
        receipt
            .state_fingerprints
            .insert(identity.resource_id.clone(), state_fp);
        receipt.resources.push(identity);
        Ok(receipt)
    }
}
impl ModuleSetup for NativeSetup {
    fn prepare(
        &self,
        module: &ModuleId,
        existing: Option<&RegistryEntry>,
        _inventory: &[NativeArtifact],
        supplied: &ModuleSettings,
    ) -> Result<SetupPlan> {
        if module != &self.module
            || supplied.schema_version != FormatVersion(1)
            || existing.is_some_and(|entry| entry.ownership != Ownership::Owned)
        {
            return Err(preview_error(
                module,
                ErrorCode::OperationDenied,
                "owned setup identity/format mismatch",
            ));
        }
        Ok(SetupPlan {
            module_id: module.clone(),
            expected_registry_revision: StateRevision(0),
            steps: self.steps(),
            checkpoints: existing
                .map(|entry| entry.checkpoints.clone())
                .unwrap_or_default(),
            missing_inputs: vec![],
            required_operator_actions: vec![],
        })
    }
    fn execute_step(&self, step: &SetupStep, ctx: &OperationContext) -> Result<StepReceipt> {
        self.valid(step, ctx)?;
        ensure_mutating(ctx, self.app.control.as_ref())?;
        let result = (|| -> Result<StepReceipt> {
            match &step.kind {
                SetupStepKind::ObtainImage { service_id, digest } => {
                    let image = self.image(service_id.as_str(), ctx)?;
                    if self.images[service_id.as_str()].manifest_digest != *digest {
                        return Err(self.error(
                            ctx,
                            ErrorCode::PlanStale,
                            "setup image pin changed",
                        ));
                    }
                    let cancel = SetupCancellation {
                        control: self.app.control.as_ref(),
                        id: &ctx.cancellation,
                    };
                    let facts = self
                        .host
                        .docker_control()
                        .pull(&image, self.budget(ctx)?, &cancel)
                        .map_err(|_| {
                            self.error(
                                ctx,
                                ErrorCode::RecoveryRequired,
                                "image acquisition uncertain; inspect before retry",
                            )
                        })?;
                    if facts.architecture != "amd64" || std::env::consts::ARCH != "x86_64" {
                        return Err(self.error(
                            ctx,
                            ErrorCode::ModuleUnsupported,
                            "R1 setup image platform is not qualified",
                        ));
                    }
                    self.observe_step(step, ctx)
                }
                SetupStepKind::RegisterOwnedResource { resource_id } => {
                    let name = resource_id
                        .as_str()
                        .strip_prefix("engine.")
                        .ok_or_else(|| {
                            self.error(ctx, ErrorCode::OperationDenied, "unknown setup resource")
                        })?;
                    let runtime = self.app.work.child("runtime").map_err(|_| {
                        self.error(ctx, ErrorCode::IoUnavailable, "runtime tree unavailable")
                    })?;
                    runtime
                        .child("services")
                        .and_then(|dir| dir.create_child(name))
                        .map_err(|_| {
                            self.error(
                                ctx,
                                ErrorCode::IoUnavailable,
                                "service scoped runtime tree creation failed",
                            )
                        })?;
                    let image = self.image(name, ctx)?;
                    let engine = Self::engine(name).ok_or_else(|| {
                        self.error(ctx, ErrorCode::OperationDenied, "unsupported owned engine")
                    })?;
                    let cancel = SetupCancellation {
                        control: self.app.control.as_ref(),
                        id: &ctx.cancellation,
                    };
                    let id = self
                        .host
                        .docker_control()
                        .create_parked(
                            self.app.installation.instance_id.as_str(),
                            name,
                            &image,
                            engine,
                            self.budget(ctx)?,
                            &cancel,
                        )
                        .map_err(|_| {
                            self.error(
                                ctx,
                                ErrorCode::RecoveryRequired,
                                "parked resource creation uncertain; inspect durable setup intent",
                            )
                        })?;
                    self.fresh_parked_receipt(step, name, &id, ctx)
                }
                SetupStepKind::SeedMissingConfig { .. } => {
                    let modules = self.app.config.child("modules").map_err(|_| {
                        self.error(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "module settings inventory unavailable",
                        )
                    })?;
                    super::installation::seed(&modules,"vless.yaml",b"schemaVersion: 1\nmoduleId: vless\nsettings:\n  transport: ws\n  path:\n    secretRef: vless/path\n",ctx,self.app.control.as_ref())?;
                    self.observe_step(step, ctx)
                }
                SetupStepKind::VerifyCompatibility { .. } => self.observe_step(step, ctx),
                _ => Err(self.error(ctx, ErrorCode::OperationDenied, "unsupported setup action")),
            }
        })();
        match result {
            Ok(receipt) => Ok(receipt),
            Err(error) => {
                let mut receipt = self.receipt(step, StepStatus::Uncertain);
                receipt.error = Some(error);
                Ok(receipt)
            }
        }
    }
}
impl SetupBackend for NativeSetup {
    fn observe_step(&self, step: &SetupStep, ctx: &OperationContext) -> Result<StepReceipt> {
        self.valid(step, ctx)?;
        match &step.kind {
            SetupStepKind::ObtainImage { service_id, digest } => {
                if self
                    .images
                    .get(service_id.as_str())
                    .is_none_or(|image| image.manifest_digest != *digest)
                {
                    return Err(self.error(
                        ctx,
                        ErrorCode::PlanStale,
                        "setup image binding changed",
                    ));
                }
                let cancel = SetupCancellation {
                    control: self.app.control.as_ref(),
                    id: &ctx.cancellation,
                };
                let cached = self
                    .host
                    .docker()
                    .cached_image(
                        &self.image(service_id.as_str(), ctx)?,
                        self.budget(ctx)?,
                        &cancel,
                    )
                    .map_err(|_| {
                        self.error(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "image cache observation unavailable",
                        )
                    })?;
                let Some(facts) = cached else {
                    return Ok(self.receipt(step, StepStatus::Prepared));
                };
                if facts.architecture != "amd64" || std::env::consts::ARCH != "x86_64" {
                    return Err(self.error(
                        ctx,
                        ErrorCode::ModuleUnsupported,
                        "R1 image platform not qualified",
                    ));
                }
                let mut receipt = self.receipt(step, StepStatus::Verified);
                receipt
                    .retained_artifacts
                    .push(ArtifactId::new(format!("image.{service_id}")).expect("fixed ID"));
                Ok(receipt)
            }
            SetupStepKind::RegisterOwnedResource { resource_id } => self.resource_receipt(
                step,
                resource_id
                    .as_str()
                    .strip_prefix("engine.")
                    .ok_or_else(|| {
                        self.error(
                            ctx,
                            ErrorCode::OperationDenied,
                            "unknown resource namespace",
                        )
                    })?,
                ctx,
            ),
            SetupStepKind::SeedMissingConfig {
                object_id,
                template_id,
            } => {
                if object_id.as_str() != "module.vless"
                    || template_id.as_str() != "template.vless.v1"
                {
                    return Err(self.error(
                        ctx,
                        ErrorCode::OperationDenied,
                        "unknown module template",
                    ));
                }
                let mut receipt = self.receipt(
                    step,
                    if self.module_settings(ctx)?.is_some() {
                        StepStatus::Verified
                    } else {
                        StepStatus::Prepared
                    },
                );
                if receipt.status == StepStatus::Verified {
                    receipt
                        .retained_artifacts
                        .push(ArtifactId::new("config.vless.v1").expect("fixed ID"));
                }
                Ok(receipt)
            }
            SetupStepKind::VerifyCompatibility { .. } => {
                let mut receipt = self.receipt(step, StepStatus::Verified);
                for prerequisite in self
                    .steps()
                    .iter()
                    .filter(|candidate| candidate.step_id != step.step_id)
                {
                    let observed = self.observe_step(prerequisite, ctx)?;
                    if observed.status != StepStatus::Verified || observed.error.is_some() {
                        return Err(self.error(
                            ctx,
                            ErrorCode::ModuleIncomplete,
                            "setup prerequisite not independently verified",
                        ));
                    }
                }
                receipt.recorded_at = UnixMillis(self.app.control.now_ms());
                Ok(receipt)
            }
            _ => Err(self.error(
                ctx,
                ErrorCode::OperationDenied,
                "foreign/operator actions not in owned setup recipe",
            )),
        }
    }
    fn registration(
        &self,
        module: &ModuleId,
        existing: Option<&RegistryEntry>,
        ctx: &OperationContext,
    ) -> Result<RegistryEntry> {
        if module != &self.module {
            return Err(self.error(
                ctx,
                ErrorCode::OperationDenied,
                "setup registration module mismatch",
            ));
        }
        if let Some(existing) = existing {
            if existing.ownership != Ownership::Owned {
                return Err(self.error(
                    ctx,
                    ErrorCode::OperationDenied,
                    "cannot register foreign module as owned",
                ));
            }
            let mut entry = existing.clone();
            entry.membership = MembershipState::Installed;
            return Ok(entry);
        }
        Ok(RegistryEntry {
            module_id: module.clone(),
            ownership: Ownership::Owned,
            membership: MembershipState::Installed,
            resources: vec![],
            setup_state: SetupState::Incomplete {
                blocked_step: None,
                required_operator_action: None,
            },
            checkpoints: vec![],
            installed_artifact_versions: BTreeMap::new(),
            original_policies: BTreeMap::new(),
            adoption_scope: None,
            retained_artifacts: vec![],
        })
    }
    fn progress(
        &self,
        module: &ModuleId,
        receipt: &StepReceipt,
        ctx: &OperationContext,
    ) -> Result<SetupProgressEvidence> {
        if module != &self.module {
            return Err(self.error(
                ctx,
                ErrorCode::OperationDenied,
                "setup progress module mismatch",
            ));
        }
        let mut preconditions = RegistryPreconditions::default();
        for resource in &receipt.resources {
            let ResourceLocator::DockerContainer { container_id } = &resource.locator else {
                return Err(self.error(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "invalid owned setup locator",
                ));
            };
            let id = ContainerId::new(container_id.as_str()).map_err(|_| {
                self.error(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "invalid owned setup identity",
                )
            })?;
            let (fresh, observation, _) = self.observed(resource.service_id.as_str(), &id, ctx)?;
            if fresh != *resource {
                return Err(self.error(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "setup resource changed before registry publication",
                ));
            }
            preconditions
                .observed
                .insert(resource.resource_id.clone(), observation);
        }
        let mut installed_versions = BTreeMap::new();
        if receipt.status == StepStatus::Verified {
            for artifact in &receipt.retained_artifacts {
                if let Some(service) = artifact.as_str().strip_prefix("image.") {
                    let pin = self.images.get(service).ok_or_else(|| {
                        self.error(
                            ctx,
                            ErrorCode::OperationDenied,
                            "unplanned setup image artifact",
                        )
                    })?;
                    installed_versions.insert(artifact.clone(), pin.manifest_digest.clone());
                }
            }
        }
        Ok(SetupProgressEvidence {
            resources: receipt.resources.clone(),
            installed_versions,
            preconditions,
        })
    }
}
impl SetupInventory for NativeSetup {
    fn artifacts(&self, _: &ModuleId, _: &OperationContext) -> Result<Vec<NativeArtifact>> {
        Ok(vec![])
    }
    fn settings(&self, module: &ModuleId, ctx: &OperationContext) -> Result<ModuleSettings> {
        if module != &self.module {
            return Err(self.error(
                ctx,
                ErrorCode::OperationDenied,
                "setup settings module mismatch",
            ));
        }
        if module.as_str() == "vless" {
            if let Some(settings) = self.module_settings(ctx)? {
                return Ok(settings);
            }
        }
        Ok(ModuleSettings {
            schema_version: FormatVersion(1),
            settings: BTreeMap::new(),
        })
    }
}
fn coordinator(
    app: &HostApplication,
    module: ModuleId,
    ctx: &OperationContext,
) -> Result<SetupCoordinator> {
    let provider = Arc::new(NativeSetup::new(app, module.clone(), ctx)?);
    Ok(SetupCoordinator {
        catalogue: Arc::new(ModuleCatalogue::builtin().map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "trusted catalogue unavailable",
            )
        })?),
        registry: app.registry.clone(),
        journal: app.journal.clone(),
        inventory: provider.clone(),
        providers: BTreeMap::from([(module, provider as Arc<dyn SetupBackend>)]),
        instance_id: app.installation.instance_id.clone(),
    })
}
pub fn complete_core(app: &HostApplication, ctx: &OperationContext) -> Result<SetupResult> {
    let coordinator = coordinator(app, ModuleId::new("core").expect("fixed ID"), ctx)?;
    let plan = coordinator.preview(&ModuleId::new("core").expect("fixed ID"), ctx)?;
    coordinator.resume(&plan, ctx)
}
pub fn add_module(
    app: &HostApplication,
    id: &ModuleId,
    ctx: &OperationContext,
) -> Result<SetupResult> {
    if id.as_str() != "vless" {
        return Err(journal::failure(
            ctx,
            ErrorCode::ModuleUnsupported,
            "only vless has a qualified owned R1 setup provider; foreign Amnezia requires explicit association",
        ));
    }
    let state = app.registry(ctx)?;
    if !state.entries.contains_key(id) {
        let desired = app.desired(ctx)?;
        let root = desired
            .documents
            .iter()
            .find(|doc| doc.object_id.as_str() == "root")
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigSyntax,
                    "new module requires coherent root config",
                )
            })?;
        let source = config::FsConfigSource::open(&app.paths.config).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "configuration source unavailable",
            )
        })?;
        let candidate = selection::prepare(
            &source,
            BTreeSet::from([id.clone()]),
            false,
            &root.revision,
            ctx,
        )?;
        SelectionWriter::new(app.config.clone(), app.journal.clone()).commit(
            &candidate,
            &root.revision,
            ctx,
        )?;
    }
    let coordinator = coordinator(app, id.clone(), ctx)?;
    let plan = coordinator.preview(id, ctx)?;
    coordinator.resume(&plan, ctx)
}

/// Read-only setup plan; no selection/config/native acquisition is performed.
pub fn preview_module(
    app: &HostApplication,
    id: &ModuleId,
    ctx: &OperationContext,
) -> Result<SetupPlan> {
    let coordinator = coordinator(app, id.clone(), ctx)?;
    coordinator.preview(id, ctx)
}

/// Shared immutable image reference inventory; only matching qualified Linux platform aliases.
pub fn locked_image_references(
    ctx: &OperationContext,
) -> Result<BTreeMap<ImageDigest, ImageReference>> {
    let lock: ImageLock = serde_json::from_str(include_str!(
        "../../../../platform/images.lock.json"
    ))
    .map_err(|_| journal::failure(ctx, ErrorCode::ConfigSyntax, "trusted image lock invalid"))?;
    if lock.schema_version != 1 {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigUnsupportedVersion,
            "trusted image lock version invalid",
        ));
    }
    let mut refs = BTreeMap::new();
    for image in lock.images.values() {
        let reference = ImageReference::new(image.reference.clone()).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "trusted image reference invalid",
            )
        })?;
        let (repository, pin) = image.reference.rsplit_once('@').ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "trusted image is not digest pinned",
            )
        })?;
        if pin != image.manifest_digest.as_str() {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "trusted image pin mismatch",
            ));
        }
        refs.insert(image.manifest_digest.clone(), reference);
        for platform in image
            .platforms
            .iter()
            .filter(|platform| platform.os == "linux" && platform.architecture == "amd64")
        {
            refs.insert(
                platform.manifest_digest.clone(),
                ImageReference::new(format!(
                    "{repository}@{}",
                    platform.manifest_digest.as_str()
                ))
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "trusted platform image reference invalid",
                    )
                })?,
            );
        }
    }
    Ok(refs)
}
pub fn locked_image_reference(name: &str, ctx: &OperationContext) -> Result<ImageReference> {
    let lock: ImageLock = serde_json::from_str(include_str!(
        "../../../../platform/images.lock.json"
    ))
    .map_err(|_| journal::failure(ctx, ErrorCode::ConfigSyntax, "trusted image lock invalid"))?;
    if lock.schema_version != 1 {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigUnsupportedVersion,
            "trusted image lock version invalid",
        ));
    }
    let image = lock.images.get(name).ok_or_else(|| {
        journal::failure(
            ctx,
            ErrorCode::ModuleUnsupported,
            "selected image is outside trusted inventory",
        )
    })?;
    let references = locked_image_references(ctx)?;
    references
        .get(&image.manifest_digest)
        .cloned()
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "selected image lock is inconsistent",
            )
        })
}
