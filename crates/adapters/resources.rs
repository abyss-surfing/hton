//! Fixed resource setup recipes. Host acquisition/seed mechanics are trusted I3 ports.
use crate::journal::{OperationControl, ensure_mutating, failure};
use crate::setup::{SetupBackend, SetupProgressEvidence};
use hton_contracts::{config::*, operations::*};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct ResourceRecipe {
    pub module_id: ModuleId,
    pub revision: Sha256Digest,
    pub images: BTreeMap<ServiceId, ImageDigest>,
    pub templates: BTreeMap<ObjectId, ArtifactId>,
    pub stopped_resources: BTreeSet<ResourceId>,
    pub resource_fingerprints: BTreeMap<ResourceId, Sha256Digest>,
    pub compatibility: CapabilityId,
    pub required_inputs: Vec<MissingInput>,
    pub operator_actions: BTreeSet<StepId>,
}
/// Each method resolves only trusted logical IDs from the injected recipe. It must
/// return a bounded observed receipt; no arbitrary image/path/command is accepted.
pub trait ResourcePort {
    fn observe(&self, step: &SetupStep, ctx: &OperationContext) -> Result<StepReceipt>;
    fn obtain_image(
        &self,
        step: &SetupStep,
        service: &ServiceId,
        digest: &ImageDigest,
        ctx: &OperationContext,
    ) -> Result<StepReceipt>;
    fn seed_missing(
        &self,
        step: &SetupStep,
        object: &ObjectId,
        template: &ArtifactId,
        ctx: &OperationContext,
    ) -> Result<StepReceipt>;
    fn prepare_stopped_resource(
        &self,
        step: &SetupStep,
        resource: &ResourceId,
        expected_fingerprint: &Sha256Digest,
        ctx: &OperationContext,
    ) -> Result<StepReceipt>;
    fn verify_compatibility(
        &self,
        step: &SetupStep,
        capability: &CapabilityId,
        ctx: &OperationContext,
    ) -> Result<StepReceipt>;
    fn operator_action(
        &self,
        step: &SetupStep,
        action: &StepId,
        ctx: &OperationContext,
    ) -> Result<StepReceipt>;
    fn registration(
        &self,
        recipe: &ResourceRecipe,
        existing: Option<&RegistryEntry>,
        ctx: &OperationContext,
    ) -> Result<RegistryEntry>;
    fn progress(
        &self,
        recipe: &ResourceRecipe,
        receipt: &StepReceipt,
        ctx: &OperationContext,
    ) -> Result<SetupProgressEvidence>;
}
pub struct ResourceAdapter {
    recipe: ResourceRecipe,
    port: Arc<dyn ResourcePort>,
    control: Arc<dyn OperationControl>,
}
impl ResourceAdapter {
    pub fn new(
        recipe: ResourceRecipe,
        port: Arc<dyn ResourcePort>,
        control: Arc<dyn OperationControl>,
    ) -> std::result::Result<Self, InvalidIdentifier> {
        if recipe.images.len() > 32
            || recipe.templates.len() > 64
            || recipe.stopped_resources.len() > 64
            || recipe.required_inputs.len() > 128
            || recipe.operator_actions.len() > 64
        {
            return Err(InvalidIdentifier);
        }
        if recipe
            .resource_fingerprints
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            != recipe.stopped_resources
        {
            return Err(InvalidIdentifier);
        }
        Ok(Self {
            recipe,
            port,
            control,
        })
    }
    fn steps(&self) -> Vec<SetupStep> {
        let mut steps = Vec::new();
        for (service_id, digest) in &self.recipe.images {
            steps.push(self.step(
                SetupStepKind::ObtainImage {
                    service_id: service_id.clone(),
                    digest: digest.clone(),
                },
                BTreeSet::new(),
            ));
        }
        for (object_id, template_id) in &self.recipe.templates {
            steps.push(self.step(
                SetupStepKind::SeedMissingConfig {
                    object_id: object_id.clone(),
                    template_id: template_id.clone(),
                },
                BTreeSet::new(),
            ));
        }
        for action_id in &self.recipe.operator_actions {
            steps.push(self.step(
                SetupStepKind::RequestOperatorAction {
                    action_id: action_id.clone(),
                },
                BTreeSet::new(),
            ));
        }
        let prerequisites: BTreeSet<_> = steps.iter().map(|s| s.step_id.clone()).collect();
        for resource_id in &self.recipe.stopped_resources {
            steps.push(self.step(
                SetupStepKind::RegisterOwnedResource {
                    resource_id: resource_id.clone(),
                },
                prerequisites.clone(),
            ));
        }
        let all: BTreeSet<_> = steps.iter().map(|s| s.step_id.clone()).collect();
        steps.push(self.step(
            SetupStepKind::VerifyCompatibility {
                capability_id: self.recipe.compatibility.clone(),
            },
            all,
        ));
        steps
    }
    fn step(&self, kind: SetupStepKind, dependencies: BTreeSet<StepId>) -> SetupStep {
        let bytes = serde_json::to_vec(&(
            &self.recipe.module_id,
            &self.recipe.revision,
            &self.recipe.resource_fingerprints,
            &kind,
        ))
        .expect("fixed data serialization");
        let digest = format!("{:x}", Sha256::digest(&bytes));
        SetupStep {
            step_id: StepId::new(format!("setup.{}", &digest[..32])).expect("fixed canonical ID"),
            idempotency_key: RequestId::new(format!("setup.{digest}")).expect("fixed canonical ID"),
            kind, dependencies, timeout: DurationMillis(120_000), required_approval: None,
            operator_alternative: "Inspect the named setup step with the local CLI; supply missing values or perform its documented native action, then resume add.".into(),
        }
    }
    fn validate_step(&self, step: &SetupStep, ctx: &OperationContext) -> Result<()> {
        if !self.steps().contains(step) {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "setup step is outside its trusted fixed recipe",
            ));
        }
        Ok(())
    }
}
impl ModuleSetup for ResourceAdapter {
    fn prepare(
        &self,
        module_id: &ModuleId,
        existing: Option<&RegistryEntry>,
        _inventory: &[NativeArtifact],
        supplied: &ModuleSettings,
    ) -> Result<SetupPlan> {
        if module_id != &self.recipe.module_id || supplied.schema_version != FormatVersion(1) {
            return Err(preview_error(
                ErrorCode::ConfigSemanticConflict,
                "resource recipe identity or format mismatch",
            ));
        }
        if existing.is_some_and(|e| e.ownership != Ownership::Owned) {
            return Err(preview_error(
                ErrorCode::OperationDenied,
                "owned resource recipe cannot manage a foreign stack",
            ));
        }
        let steps = self.steps();
        let operator = steps
            .iter()
            .filter(|s| matches!(s.kind, SetupStepKind::RequestOperatorAction { .. }))
            .map(|s| s.step_id.clone())
            .collect();
        let missing = self
            .recipe
            .required_inputs
            .iter()
            .filter(|input| missing_input(supplied, &input.path))
            .cloned()
            .collect();
        Ok(SetupPlan {
            module_id: module_id.clone(),
            expected_registry_revision: StateRevision(0),
            steps,
            checkpoints: existing.map(|e| e.checkpoints.clone()).unwrap_or_default(),
            missing_inputs: missing,
            required_operator_actions: operator,
        })
    }
    fn execute_step(&self, step: &SetupStep, context: &OperationContext) -> Result<StepReceipt> {
        ensure_mutating(context, self.control.as_ref())?;
        self.validate_step(step, context)?;
        let receipt = match &step.kind {
            SetupStepKind::ObtainImage { service_id, digest } => {
                self.port.obtain_image(step, service_id, digest, context)
            }
            SetupStepKind::SeedMissingConfig {
                object_id,
                template_id,
            } => self
                .port
                .seed_missing(step, object_id, template_id, context),
            SetupStepKind::RegisterOwnedResource { resource_id } => {
                self.port.prepare_stopped_resource(
                    step,
                    resource_id,
                    self.recipe
                        .resource_fingerprints
                        .get(resource_id)
                        .expect("recipe key invariant"),
                    context,
                )
            }
            SetupStepKind::VerifyCompatibility { capability_id } => {
                self.port.verify_compatibility(step, capability_id, context)
            }
            SetupStepKind::RequestOperatorAction { action_id } => {
                self.port.operator_action(step, action_id, context)
            }
            SetupStepKind::AssociateForeignResource { .. } => Err(failure(
                context,
                ErrorCode::OperationDenied,
                "owned recipe cannot associate foreign resources",
            )),
        }?;
        if receipt.step_id != step.step_id
            || receipt.idempotency_key != step.idempotency_key
            || receipt.resources.iter().any(|r| {
                self.recipe.resource_fingerprints.get(&r.resource_id) != Some(&r.fingerprint)
            })
        {
            return Err(failure(
                context,
                ErrorCode::ResourceDrift,
                "resource provider returned another step receipt",
            ));
        }
        Ok(receipt)
    }
}
impl SetupBackend for ResourceAdapter {
    fn observe_step(&self, step: &SetupStep, ctx: &OperationContext) -> Result<StepReceipt> {
        self.validate_step(step, ctx)?;
        let receipt = self.port.observe(step, ctx)?;
        if receipt
            .resources
            .iter()
            .any(|r| self.recipe.resource_fingerprints.get(&r.resource_id) != Some(&r.fingerprint))
        {
            return Err(failure(
                ctx,
                ErrorCode::ResourceDrift,
                "observed resource differs from the immutable setup recipe",
            ));
        }
        Ok(receipt)
    }
    fn registration(
        &self,
        module: &ModuleId,
        existing: Option<&RegistryEntry>,
        ctx: &OperationContext,
    ) -> Result<RegistryEntry> {
        if module != &self.recipe.module_id {
            return Err(failure(
                ctx,
                ErrorCode::ModuleUnknown,
                "resource recipe module mismatch",
            ));
        }
        let entry = self.port.registration(&self.recipe, existing, ctx)?;
        if entry.ownership != Ownership::Owned
            || entry.module_id != *module
            || entry.adoption_scope.is_some()
            || entry.resources.iter().any(|r| {
                self.recipe.resource_fingerprints.get(&r.resource_id) != Some(&r.fingerprint)
            })
            || entry
                .installed_artifact_versions
                .values()
                .any(|version| !self.recipe.images.values().any(|pin| pin == version))
        {
            return Err(failure(
                ctx,
                ErrorCode::ResourceDrift,
                "resource registration ownership mismatch",
            ));
        }
        Ok(entry)
    }
    fn progress(
        &self,
        module: &ModuleId,
        receipt: &StepReceipt,
        ctx: &OperationContext,
    ) -> Result<SetupProgressEvidence> {
        if module != &self.recipe.module_id {
            return Err(failure(
                ctx,
                ErrorCode::ModuleUnknown,
                "resource progress module mismatch",
            ));
        }
        let progress = self.port.progress(&self.recipe, receipt, ctx)?;
        if progress
            .resources
            .iter()
            .any(|r| self.recipe.resource_fingerprints.get(&r.resource_id) != Some(&r.fingerprint))
            || progress
                .installed_versions
                .values()
                .any(|digest| !self.recipe.images.values().any(|pin| pin == digest))
        {
            return Err(failure(
                ctx,
                ErrorCode::ResourceDrift,
                "resource progress escapes its fixed recipe",
            ));
        }
        Ok(progress)
    }
}
fn missing_input(settings: &ModuleSettings, path: &ConfigPath) -> bool {
    let mut components = path.0.iter().map(ObjectId::as_str);
    let Some(first) = components.next() else {
        return true;
    };
    let first = if first == "settings" {
        components.next().unwrap_or("")
    } else {
        first
    };
    let mut value = settings.settings.get(first);
    for key in components {
        value = value.and_then(|v| v.get(key));
    }
    value.is_none_or(|v| v.is_null() || v.as_str().is_some_and(str::is_empty))
}
fn preview_error(code: ErrorCode, message: &'static str) -> Error {
    Error {
        code,
        message: message.into(),
        module_id: None,
        config_path: None,
        resource_id: None,
        operation_id: OperationId::new("setup-preview").expect("fixed ID"),
        retryability: Retryability::AfterRepair,
        repair_hint: None,
        retained_state: RetainedState::Unchanged,
    }
}
