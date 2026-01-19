mod asset_resolver;
mod error;
mod expression;
mod interaction;
mod model;
mod render;
mod state_store;

pub use asset_resolver::{
    register_host_asset_callback, register_host_asset_map, register_host_asset_resolver,
};
pub use error::ComponentError;
pub use interaction::handle_interaction;
pub use model::*;
pub use render::render_card;

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
                "component": "schemas/component.schema.json",
                "input": "schemas/io/input.schema.json",
                "output": "schemas/io/output.schema.json"
            }
        }
    })
    .to_string()
}

pub fn handle_message(operation: &str, input: &str) -> String {
    match parse_invocation(input) {
        Ok(mut invocation) => {
            // Allow the operation name to steer mode selection if the host provides it.
            if operation.eq_ignore_ascii_case("validate") {
                invocation.mode = InvocationMode::Validate;
            }
            match handle_invocation(invocation) {
                Ok(result) => serde_json::to_string(&result)
                    .unwrap_or_else(|err| error_payload(&format!("serialization error: {err}"))),
                Err(err) => error_payload(&err.to_string()),
            }
        }
        Err(err) => error_payload(&format!("invalid invocation: {err}")),
    }
}

pub fn handle_invocation(
    mut invocation: AdaptiveCardInvocation,
) -> Result<AdaptiveCardResult, ComponentError> {
    state_store::load_state_if_missing(&mut invocation, None)?;
    if let Some(interaction) = invocation.interaction.as_ref()
        && interaction.enabled == Some(false)
    {
        invocation.interaction = None;
    }
    if invocation.interaction.is_some() {
        return handle_interaction(&invocation);
    }

    let rendered = render_card(&invocation)?;
    let rendered_card = match invocation.mode {
        InvocationMode::Validate => None,
        InvocationMode::Render | InvocationMode::RenderAndValidate => Some(rendered.card),
    };

    Ok(AdaptiveCardResult {
        rendered_card,
        event: None,
        state_updates: Vec::new(),
        session_updates: Vec::new(),
        card_features: rendered.features,
        validation_issues: rendered.validation_issues,
        telemetry_events: Vec::new(),
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
    node_id: Option<String>,
    #[serde(default)]
    envelope: Option<greentic_types::InvocationEnvelope>,
}

fn parse_invocation(input: &str) -> Result<AdaptiveCardInvocation, ComponentError> {
    let value: serde_json::Value = serde_json::from_str(input)?;
    if let Some(invocation_value) = find_invocation_value(&value) {
        return serde_json::from_value::<AdaptiveCardInvocation>(invocation_value)
            .map_err(ComponentError::Serde);
    }

    if let Some(inner) = value.get("config") {
        if let Ok(invocation) = serde_json::from_value::<AdaptiveCardInvocation>(inner.clone()) {
            return merge_envelope(invocation, &value);
        }
        if let Some(card) = inner.get("card")
            && let Ok(invocation) = serde_json::from_value::<AdaptiveCardInvocation>(card.clone())
        {
            return merge_envelope(invocation, &value);
        }
    }

    let mut env: InvocationEnvelope = serde_json::from_value(value)?;
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
    if let Some(envelope) = env.get("envelope") {
        inv.envelope = serde_json::from_value(envelope.clone()).ok();
    }
    Ok(inv)
}

fn find_invocation_value(value: &serde_json::Value) -> Option<serde_json::Value> {
    let obj = value.as_object()?;
    if obj.contains_key("card_source") || obj.contains_key("card_spec") {
        return Some(value.clone());
    }
    if let Some(inv) = obj.get("invocation") {
        return Some(inv.clone());
    }
    if let Some(card) = obj.get("card") {
        return Some(card.clone());
    }
    if let Some(payload) = obj.get("payload")
        && payload
            .as_object()
            .map(|p| p.contains_key("card_source") || p.contains_key("card_spec"))
            .unwrap_or(false)
    {
        return Some(payload.clone());
    }
    if let Some(config) = obj.get("config") {
        if config
            .as_object()
            .map(|c| c.contains_key("card_source") || c.contains_key("card_spec"))
            .unwrap_or(false)
        {
            return Some(config.clone());
        }
        if let Some(card) = config.get("card") {
            return Some(card.clone());
        }
    }
    None
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
    if env.envelope.is_some() {
        inv.envelope = env.envelope;
    }
    inv
}

fn error_payload(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}
