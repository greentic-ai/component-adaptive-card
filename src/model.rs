use greentic_types::InvocationEnvelope;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CardSource {
    #[default]
    Inline,
    Asset,
    Catalog,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CardSpec {
    pub inline_json: Option<Value>,
    pub asset_path: Option<String>,
    pub catalog_name: Option<String>,
    pub template_params: Option<Value>,
    pub asset_registry: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum InvocationMode {
    Render,
    Validate,
    #[default]
    RenderAndValidate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ValidationMode {
    Off,
    #[default]
    Warn,
    Error,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdaptiveCardInvocation {
    #[serde(default)]
    #[serde(alias = "card_source")]
    pub card_source: CardSource,
    #[serde(default)]
    #[serde(alias = "card_spec")]
    pub card_spec: CardSpec,

    #[serde(default)]
    #[serde(alias = "node_id")]
    pub node_id: Option<String>,

    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub session: Value,
    #[serde(default)]
    pub state: Value,

    #[serde(default)]
    pub interaction: Option<CardInteraction>,

    #[serde(default)]
    pub mode: InvocationMode,

    #[serde(default)]
    #[serde(alias = "validation_mode")]
    pub validation_mode: ValidationMode,

    /// Optional shared invocation envelope metadata from the host.
    #[serde(default)]
    pub envelope: Option<InvocationEnvelope>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "PascalCase")]
pub enum CardInteractionType {
    #[default]
    Submit,
    Execute,
    OpenUrl,
    ShowCard,
    ToggleVisibility,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardInteraction {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(alias = "interaction_type")]
    pub interaction_type: CardInteractionType,
    #[serde(alias = "action_id")]
    pub action_id: String,
    #[serde(default)]
    pub verb: Option<String>,
    #[serde(alias = "raw_inputs")]
    #[serde(default)]
    pub raw_inputs: Value,
    #[serde(alias = "card_instance_id")]
    pub card_instance_id: String,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "PascalCase")]
pub enum AdaptiveActionType {
    #[default]
    Submit,
    Execute,
    OpenUrl,
    ShowCard,
    ToggleVisibility,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdaptiveActionEvent {
    pub action_type: AdaptiveActionType,
    pub action_id: String,
    #[serde(default)]
    pub verb: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub inputs: Value,

    pub card_id: String,
    pub card_instance_id: String,
    #[serde(default)]
    pub subcard_id: Option<String>,

    #[serde(default)]
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum StateUpdateOp {
    Set { path: String, value: Value },
    Merge { path: String, value: Value },
    Delete { path: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum SessionUpdateOp {
    SetRoute { route: String },
    SetAttribute { key: String, value: Value },
    DeleteAttribute { key: String },
    PushCardStack { card_id: String },
    PopCardStack,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardFeatureSummary {
    pub version: Option<String>,
    pub used_elements: Vec<String>,
    pub used_actions: Vec<String>,
    pub uses_show_card: bool,
    pub uses_toggle_visibility: bool,
    pub uses_media: bool,
    pub uses_auth: bool,
    #[serde(default)]
    pub requires_features: Value,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub code: String,
    pub message: String,
    pub path: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryEvent {
    pub name: String,
    #[serde(default)]
    pub properties: Value,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdaptiveCardResult {
    pub rendered_card: Option<Value>,
    pub event: Option<AdaptiveActionEvent>,
    #[serde(default)]
    pub state_updates: Vec<StateUpdateOp>,
    #[serde(default)]
    pub session_updates: Vec<SessionUpdateOp>,
    pub card_features: CardFeatureSummary,
    #[serde(default)]
    pub validation_issues: Vec<ValidationIssue>,
    #[serde(default)]
    pub telemetry_events: Vec<TelemetryEvent>,
}
