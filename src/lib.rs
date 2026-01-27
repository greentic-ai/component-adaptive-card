mod asset_resolver;
mod error;
mod expression;
mod interaction;
mod model;
mod render;
mod state_store;
mod trace;
mod validation;

use once_cell::sync::Lazy;

pub use asset_resolver::{
    register_host_asset_callback, register_host_asset_map, register_host_asset_resolver,
};
pub use error::ComponentError;
pub use interaction::handle_interaction;
pub use model::*;
pub use render::render_card;

static COMPONENT_SCHEMA_JSON: Lazy<serde_json::Value> = Lazy::new(|| {
    serde_json::from_str(include_str!("../schemas/component.schema.json"))
        .expect("failed to parse component schema")
});
static INPUT_SCHEMA_JSON: Lazy<serde_json::Value> = Lazy::new(|| {
    serde_json::from_str(include_str!("../schemas/io/input.schema.json"))
        .expect("failed to parse input schema")
});
static OUTPUT_SCHEMA_JSON: Lazy<serde_json::Value> = Lazy::new(|| {
    serde_json::from_str(include_str!("../schemas/io/output.schema.json"))
        .expect("failed to parse output schema")
});

#[cfg(target_arch = "wasm32")]
#[used]
#[unsafe(link_section = ".greentic.wasi")]
static WASI_TARGET_MARKER: [u8; 13] = *b"wasm32-wasip2";

#[cfg(target_arch = "wasm32")]
mod component {
    use greentic_interfaces_guest::component::node::{
        self, ExecCtx, InvokeResult, LifecycleStatus, StreamEvent,
    };

    use super::{describe_payload, handle_message};

    pub(super) struct Component;

    impl node::Guest for Component {
        fn get_manifest() -> String {
            describe_payload()
        }

        fn on_start(_ctx: ExecCtx) -> Result<LifecycleStatus, String> {
            Ok(LifecycleStatus::Ok)
        }

        fn on_stop(_ctx: ExecCtx, _reason: String) -> Result<LifecycleStatus, String> {
            Ok(LifecycleStatus::Ok)
        }

        fn invoke(_ctx: ExecCtx, op: String, input: String) -> InvokeResult {
            InvokeResult::Ok(handle_message(&op, &input))
        }

        fn invoke_stream(_ctx: ExecCtx, op: String, input: String) -> Vec<StreamEvent> {
            vec![
                StreamEvent::Progress(0),
                StreamEvent::Data(handle_message(&op, &input)),
                StreamEvent::Done,
            ]
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod exports {
    use super::component::Component;
    use greentic_interfaces_guest::component::node;

    #[unsafe(export_name = "greentic:component/node@0.5.0#get-manifest")]
    unsafe extern "C" fn export_get_manifest() -> *mut u8 {
        unsafe { node::_export_get_manifest_cabi::<Component>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#get-manifest")]
    unsafe extern "C" fn post_return_get_manifest(arg0: *mut u8) {
        unsafe { node::__post_return_get_manifest::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/node@0.5.0#on-start")]
    unsafe extern "C" fn export_on_start(arg0: *mut u8) -> *mut u8 {
        unsafe { node::_export_on_start_cabi::<Component>(arg0) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#on-start")]
    unsafe extern "C" fn post_return_on_start(arg0: *mut u8) {
        unsafe { node::__post_return_on_start::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/node@0.5.0#on-stop")]
    unsafe extern "C" fn export_on_stop(arg0: *mut u8) -> *mut u8 {
        unsafe { node::_export_on_stop_cabi::<Component>(arg0) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#on-stop")]
    unsafe extern "C" fn post_return_on_stop(arg0: *mut u8) {
        unsafe { node::__post_return_on_stop::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/node@0.5.0#invoke")]
    unsafe extern "C" fn export_invoke(arg0: *mut u8) -> *mut u8 {
        unsafe { node::_export_invoke_cabi::<Component>(arg0) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#invoke")]
    unsafe extern "C" fn post_return_invoke(arg0: *mut u8) {
        unsafe { node::__post_return_invoke::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/node@0.5.0#invoke-stream")]
    unsafe extern "C" fn export_invoke_stream(arg0: *mut u8) -> *mut u8 {
        unsafe { node::_export_invoke_stream_cabi::<Component>(arg0) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#invoke-stream")]
    unsafe extern "C" fn post_return_invoke_stream(arg0: *mut u8) {
        unsafe { node::__post_return_invoke_stream::<Component>(arg0) };
    }
}

pub fn describe_payload() -> String {
    serde_json::json!({
        "component": {
            "name": "component-adaptive-card",
            "org": "ai.greentic",
            "version": "0.1.2",
            "world": "greentic:component/component@0.5.0",
            "schemas": {
                "component": COMPONENT_SCHEMA_JSON.clone(),
                "input": INPUT_SCHEMA_JSON.clone(),
                "output": OUTPUT_SCHEMA_JSON.clone()
            }
        }
    })
    .to_string()
}

pub fn handle_message(operation: &str, input: &str) -> String {
    let value: serde_json::Value = match serde_json::from_str(input) {
        Ok(value) => value,
        Err(err) => {
            return error_payload(
                "AC_SCHEMA_INVALID",
                "invalid JSON",
                Some(serde_json::Value::String(err.to_string())),
            );
        }
    };
    let invocation_value =
        validation::locate_invocation_candidate(&value).unwrap_or_else(|| value.clone());
    let validation_mode = read_validation_mode(&value, &invocation_value);
    let mut validation_issues = if validation_mode == ValidationMode::Off {
        Vec::new()
    } else {
        validation::validate_invocation_schema(&invocation_value)
    };
    if validation_mode == ValidationMode::Error && !validation_issues.is_empty() {
        return validation_error_payload(&validation_issues, None);
    }

    let mut invocation = match parse_invocation_value(&value) {
        Ok(invocation) => invocation,
        Err(err) => {
            if !validation_issues.is_empty() {
                return validation_error_payload(&validation_issues, Some(&err.to_string()));
            }
            return error_payload(
                "AC_SCHEMA_INVALID",
                "invalid invocation",
                Some(serde_json::Value::String(err.to_string())),
            );
        }
    };
    eprintln!(
        "DEBUG invocation payload: {}",
        serde_json::to_string(&invocation.payload).unwrap_or_else(|_| "\"<error>\"".to_string())
    );
    // Allow the operation name to steer mode selection if the host provides it.
    if operation.eq_ignore_ascii_case("validate") {
        invocation.mode = InvocationMode::Validate;
    }
    match handle_invocation(invocation) {
        Ok(mut result) => {
            if validation_mode != ValidationMode::Off {
                result.validation_issues.append(&mut validation_issues);
            }
            serde_json::to_string(&result).unwrap_or_else(|err| {
                error_payload(
                    "AC_INTERNAL_ERROR",
                    "serialization error",
                    Some(serde_json::Value::String(err.to_string())),
                )
            })
        }
        Err(err) => {
            if !validation_issues.is_empty() {
                return validation_error_payload(&validation_issues, Some(&err.to_string()));
            }
            error_payload_from_error(&err)
        }
    }
}

pub fn handle_invocation(
    mut invocation: AdaptiveCardInvocation,
) -> Result<AdaptiveCardResult, ComponentError> {
    let state_loaded = state_store::load_state_if_missing(&mut invocation, None)?;
    let state_read_hash = state_loaded.as_ref().and_then(trace::hash_value);
    if let Some(interaction) = invocation.interaction.as_ref()
        && interaction.enabled == Some(false)
    {
        invocation.interaction = None;
    }
    if invocation.interaction.is_some() {
        return handle_interaction(&invocation);
    }

    let rendered = render_card(&invocation)?;
    if invocation.validation_mode == ValidationMode::Error && !rendered.validation_issues.is_empty()
    {
        return Err(ComponentError::CardValidation(rendered.validation_issues));
    }
    let rendered_card = match invocation.mode {
        InvocationMode::Validate => None,
        InvocationMode::Render | InvocationMode::RenderAndValidate => Some(rendered.card),
    };

    let mut telemetry_events = Vec::new();
    if trace::trace_enabled() {
        let state_key = Some(state_store::state_key_for(&invocation, None));
        telemetry_events.push(trace::build_trace_event(
            &invocation,
            &rendered.asset_resolution,
            &rendered.binding_summary,
            None,
            state_key,
            state_read_hash,
            None,
        ));
    }

    Ok(AdaptiveCardResult {
        rendered_card,
        event: None,
        state_updates: Vec::new(),
        session_updates: Vec::new(),
        card_features: rendered.features,
        validation_issues: rendered.validation_issues,
        telemetry_events,
    })
}

#[derive(serde::Deserialize, Default)]
struct InvocationEnvelope {
    #[serde(default)]
    config: Option<AdaptiveCardInvocation>,
    #[serde(default)]
    payload: serde_json::Value,
    #[serde(default)]
    session: serde_json::Value,
    #[serde(default)]
    state: serde_json::Value,
    #[serde(default)]
    interaction: Option<CardInteraction>,
    #[serde(default)]
    mode: Option<InvocationMode>,
    #[serde(default)]
    #[serde(alias = "validationMode")]
    validation_mode: Option<ValidationMode>,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    envelope: Option<greentic_types::InvocationEnvelope>,
}

fn parse_invocation_value(
    value: &serde_json::Value,
) -> Result<AdaptiveCardInvocation, ComponentError> {
    if let Some(invocation_value) = validation::locate_invocation_candidate(value) {
        return serde_json::from_value::<AdaptiveCardInvocation>(invocation_value)
            .map_err(ComponentError::Serde);
    }

    if let Some(inner) = value.get("config") {
        if let Ok(invocation) = serde_json::from_value::<AdaptiveCardInvocation>(inner.clone()) {
            return merge_envelope(invocation, value);
        }
        if let Some(card) = inner.get("card")
            && let Ok(invocation) = serde_json::from_value::<AdaptiveCardInvocation>(card.clone())
        {
            return merge_envelope(invocation, value);
        }
    }

    let mut env: InvocationEnvelope = serde_json::from_value(value.clone())?;
    if env.config.is_none()
        && let Ok(invocation) =
            serde_json::from_value::<AdaptiveCardInvocation>(env.payload.clone())
    {
        return Ok(invocation);
    }
    let config = env.config.take().unwrap_or_default();
    Ok(merge_envelope_struct(config, env))
}

fn merge_envelope(
    mut inv: AdaptiveCardInvocation,
    value: &serde_json::Value,
) -> Result<AdaptiveCardInvocation, ComponentError> {
    let env: serde_json::Value = value.clone();
    let payload = env
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let session = env
        .get("session")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let state = env.get("state").cloned().unwrap_or(serde_json::Value::Null);
    if let Some(node_id) = env.get("node_id").and_then(|v| v.as_str()) {
        inv.node_id = Some(node_id.to_string());
    }
    if !payload.is_null() {
        inv.payload = payload;
    }
    if !session.is_null() {
        inv.session = session;
    }
    if !state.is_null() {
        inv.state = state;
    }
    if inv.interaction.is_none()
        && let Some(interaction) = env.get("interaction")
    {
        inv.interaction = serde_json::from_value(interaction.clone()).ok();
    }
    if let Some(mode) = env.get("mode")
        && let Ok(parsed) = serde_json::from_value::<InvocationMode>(mode.clone())
    {
        inv.mode = parsed;
    }
    if let Some(mode_value) = env
        .get("validation_mode")
        .or_else(|| env.get("validationMode"))
        && let Some(parsed) = parse_validation_mode(mode_value)
    {
        inv.validation_mode = parsed;
    }
    if let Some(envelope) = env.get("envelope") {
        inv.envelope = serde_json::from_value(envelope.clone()).ok();
    }
    Ok(inv)
}

#[cfg(test)]
mod debug_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_payload_value() {
        let input = json!({
            "card_spec": {
                "inline_json": {
                    "type": "AdaptiveCard",
                    "version": "1.3",
                    "body": [
                        { "type": "TextBlock", "text": "@{payload.title}" }
                    ]
                }
            },
            "payload": {
                "title": "Hello"
            }
        });
        let invocation = parse_invocation_value(&input).expect("should parse");
        println!("payload: {}", invocation.payload);
    }
}

fn merge_envelope_struct(
    mut inv: AdaptiveCardInvocation,
    env: InvocationEnvelope,
) -> AdaptiveCardInvocation {
    if inv.card_spec.inline_json.is_none()
        && let Ok(candidate) = serde_json::from_value::<AdaptiveCardInvocation>(env.payload.clone())
    {
        return candidate;
    }
    if env.node_id.is_some() {
        inv.node_id = env.node_id;
    }
    if !env.payload.is_null() {
        inv.payload = env.payload;
    }
    if !env.session.is_null() {
        inv.session = env.session;
    }
    if !env.state.is_null() {
        inv.state = env.state;
    }
    if inv.interaction.is_none() {
        inv.interaction = env.interaction;
    }
    if let Some(mode) = env.mode {
        inv.mode = mode;
    }
    if let Some(mode) = env.validation_mode {
        inv.validation_mode = mode;
    }
    if env.envelope.is_some() {
        inv.envelope = env.envelope;
    }
    inv
}

fn error_payload(code: &str, message: &str, details: Option<serde_json::Value>) -> String {
    let mut payload = serde_json::Map::new();
    payload.insert(
        "code".to_string(),
        serde_json::Value::String(code.to_string()),
    );
    payload.insert(
        "message".to_string(),
        serde_json::Value::String(message.to_string()),
    );
    if let Some(details) = details {
        payload.insert("details".to_string(), details);
    }
    serde_json::json!({ "error": payload }).to_string()
}

fn validation_error_payload(issues: &[ValidationIssue], detail: Option<&str>) -> String {
    let mut message = "invocation schema validation failed".to_string();
    if let Some(detail) = detail {
        message = format!("{message}: {detail}");
    }
    let details = serde_json::json!({ "validation_issues": issues });
    error_payload("AC_SCHEMA_INVALID", &message, Some(details))
}

fn error_payload_from_error(err: &ComponentError) -> String {
    let issue_details = |code: &str, message: String, path: &str| {
        serde_json::json!({
            "validation_issues": [{
                "code": code,
                "message": message,
                "path": path
            }]
        })
    };
    match err {
        ComponentError::InvalidInput(message) => error_payload(
            "AC_SCHEMA_INVALID",
            "invalid input",
            Some(issue_details("AC_SCHEMA_INVALID", message.clone(), "/")),
        ),
        ComponentError::Serde(inner) => error_payload(
            "AC_SCHEMA_INVALID",
            "invalid input",
            Some(issue_details("AC_SCHEMA_INVALID", inner.to_string(), "/")),
        ),
        ComponentError::Io(inner) => error_payload(
            "AC_SCHEMA_INVALID",
            "io error",
            Some(issue_details("AC_SCHEMA_INVALID", inner.to_string(), "/")),
        ),
        ComponentError::AssetNotFound(path) => error_payload(
            "AC_ASSET_NOT_FOUND",
            "asset not found",
            Some(issue_details(
                "AC_ASSET_NOT_FOUND",
                path.clone(),
                "/card_spec",
            )),
        ),
        ComponentError::AssetParse(message) => error_payload(
            "AC_ASSET_PARSE_ERROR",
            "asset parse error",
            Some(issue_details(
                "AC_ASSET_PARSE_ERROR",
                message.clone(),
                "/card_spec",
            )),
        ),
        ComponentError::Asset(message) => error_payload(
            "AC_ASSET_NOT_FOUND",
            "asset error",
            Some(issue_details(
                "AC_ASSET_NOT_FOUND",
                message.clone(),
                "/card_spec",
            )),
        ),
        ComponentError::Binding(message) => error_payload(
            "AC_BINDING_EVAL_ERROR",
            "binding evaluation error",
            Some(issue_details(
                "AC_BINDING_EVAL_ERROR",
                message.clone(),
                "/card_spec/inline_json",
            )),
        ),
        ComponentError::CardValidation(issues) => {
            let details = serde_json::json!({ "validation_issues": issues });
            error_payload(
                "AC_CARD_VALIDATION_FAILED",
                "card validation failed",
                Some(details),
            )
        }
        ComponentError::InteractionInvalid(message) => error_payload(
            "AC_INTERACTION_INVALID",
            "interaction invalid",
            Some(issue_details(
                "AC_INTERACTION_INVALID",
                message.clone(),
                "/interaction",
            )),
        ),
        ComponentError::StateStore(message) => error_payload(
            "AC_SCHEMA_INVALID",
            "state store error",
            Some(issue_details(
                "AC_SCHEMA_INVALID",
                message.clone(),
                "/state",
            )),
        ),
    }
}

fn read_validation_mode(
    value: &serde_json::Value,
    invocation_value: &serde_json::Value,
) -> ValidationMode {
    invocation_value
        .get("validation_mode")
        .or_else(|| invocation_value.get("validationMode"))
        .or_else(|| value.get("validation_mode"))
        .or_else(|| value.get("validationMode"))
        .and_then(parse_validation_mode)
        .unwrap_or_default()
}

fn parse_validation_mode(value: &serde_json::Value) -> Option<ValidationMode> {
    let raw = value.as_str()?.to_ascii_lowercase();
    match raw.as_str() {
        "off" => Some(ValidationMode::Off),
        "warn" => Some(ValidationMode::Warn),
        "error" => Some(ValidationMode::Error),
        _ => None,
    }
}
