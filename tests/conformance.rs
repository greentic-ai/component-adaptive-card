use component_adaptive_card::{
    AdaptiveCardInvocation, CardInteraction, CardInteractionType, CardSource, CardSpec,
    InvocationMode, handle_invocation, register_host_asset_callback,
};
use serde_json::json;
use std::fs;

fn base_invocation(card: serde_json::Value) -> AdaptiveCardInvocation {
    AdaptiveCardInvocation {
        card_source: CardSource::Inline,
        card_spec: CardSpec {
            inline_json: Some(card),
            asset_path: None,
            catalog_name: None,
            template_params: None,
            asset_registry: None,
        },
        payload: json!({}),
        session: json!({}),
        state: json!({}),
        interaction: None,
        mode: InvocationMode::RenderAndValidate,
        envelope: None,
    }
}

#[test]
fn describe_mentions_world() {
    let payload = component_adaptive_card::describe_payload();
    let json: serde_json::Value = serde_json::from_str(&payload).expect("describe should be json");
    assert_eq!(
        json["component"]["world"],
        "greentic:component/component@0.5.0"
    );
}

#[test]
fn inline_render_returns_card_and_features() {
    let card = json!({
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            { "type": "TextBlock", "text": "Hello" }
        ]
    });
    let invocation = base_invocation(card.clone());
    let result = handle_invocation(invocation).expect("render should succeed");

    assert_eq!(result.rendered_card, Some(card));
    assert!(
        result
            .card_features
            .used_elements
            .contains(&"TextBlock".to_string())
    );
}

#[test]
fn asset_render_loads_card() {
    let spec = CardSpec {
        asset_path: Some("tests/assets/cards/simple.json".to_string()),
        ..Default::default()
    };
    let invocation = AdaptiveCardInvocation {
        card_source: CardSource::Asset,
        card_spec: spec,
        payload: json!({}),
        session: json!({}),
        state: json!({}),
        interaction: None,
        mode: InvocationMode::RenderAndValidate,
        envelope: None,
    };

    let result = handle_invocation(invocation).expect("asset render");
    let card = result.rendered_card.expect("card should render");
    assert_eq!(card["type"], "AdaptiveCard");
    assert!(
        result
            .card_features
            .used_elements
            .contains(&"TextBlock".to_string())
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn catalog_resolution_uses_env_mapping() {
    let mapping = json!({ "sample": "tests/assets/cards/simple.json" });
    let catalog_file = std::env::temp_dir().join("adaptive_card_catalog_test.json");
    fs::write(&catalog_file, serde_json::to_string(&mapping).unwrap()).unwrap();
    unsafe {
        std::env::set_var(
            "ADAPTIVE_CARD_CATALOG_FILE",
            catalog_file.to_string_lossy().to_string(),
        );
    }

    let invocation = AdaptiveCardInvocation {
        card_source: CardSource::Catalog,
        card_spec: CardSpec {
            catalog_name: Some("sample".to_string()),
            asset_registry: None,
            ..Default::default()
        },
        payload: json!({}),
        session: json!({}),
        state: json!({}),
        interaction: None,
        mode: InvocationMode::RenderAndValidate,
        envelope: None,
    };

    let result = handle_invocation(invocation).expect("catalog render");
    let card = result.rendered_card.expect("card should render");
    assert_eq!(card["type"], "AdaptiveCard");
}

#[test]
fn bindings_apply_session_and_state() {
    let card = json!({
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            { "type": "TextBlock", "text": "Hello @{session.user.name}, step ${state.step}" }
        ]
    });
    let mut invocation = base_invocation(card);
    invocation.session = json!({ "user": { "name": "Ada" }});
    invocation.state = json!({ "step": 2 });

    let result = handle_invocation(invocation).expect("render with bindings");
    let rendered = result.rendered_card.expect("card should render");
    let text = rendered["body"][0]["text"]
        .as_str()
        .expect("text should be string");
    assert_eq!(text, "Hello Ada, step 2");
}

#[test]
fn bindings_apply_default_with_coalesce() {
    let card = json!({
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            { "type": "TextBlock", "text": "Hello @{session.user.name||\"Guest\"}" }
        ]
    });
    let invocation = base_invocation(card);
    let result = handle_invocation(invocation).expect("render with default");
    let rendered = result.rendered_card.expect("card should render");
    let text = rendered["body"][0]["text"]
        .as_str()
        .expect("text should be string");
    assert_eq!(text, "Hello Guest");
}

#[test]
fn expression_placeholders_support_equality_and_ternary() {
    let card = json!({
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            { "type": "TextBlock", "text": "${payload.status == \"ok\" ? \"green\" : \"red\"}" }
        ]
    });
    let mut invocation = base_invocation(card);
    invocation.payload = json!({ "status": "ok" });
    let result = handle_invocation(invocation).expect("expression render");
    let rendered = result.rendered_card.expect("card should render");
    let text = rendered["body"][0]["text"]
        .as_str()
        .expect("text should be string");
    assert_eq!(text, "green");
}

#[test]
fn submit_interaction_emits_event_and_updates_state() {
    let card = json!({
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            { "type": "Input.Text", "id": "comment" }
        ]
    });
    let mut invocation = base_invocation(card);
    invocation.interaction = Some(CardInteraction {
        interaction_type: CardInteractionType::Submit,
        action_id: "submit-1".to_string(),
        verb: None,
        raw_inputs: json!({ "comment": "Looks good" }),
        card_instance_id: "card-1".to_string(),
        metadata: json!({ "route": "next" }),
    });

    let result = handle_invocation(invocation).expect("interaction");
    let event = result.event.expect("event should exist");
    assert_eq!(event.action_id, "submit-1");
    assert_eq!(event.inputs["comment"], "Looks good");

    assert!(result
        .state_updates
        .iter()
        .any(|op| matches!(op, component_adaptive_card::StateUpdateOp::Merge { path, .. } if path == "form_data")));
    assert!(result
        .session_updates
        .iter()
        .any(|op| matches!(op, component_adaptive_card::SessionUpdateOp::SetRoute { route } if route == "next")));
}

#[test]
fn toggle_visibility_sets_state_flag() {
    let card = json!({
        "type": "AdaptiveCard",
        "version": "1.6",
        "actions": [
            { "type": "Action.ToggleVisibility", "targetElements": ["section-1"] }
        ]
    });
    let mut invocation = base_invocation(card);
    invocation.interaction = Some(CardInteraction {
        interaction_type: CardInteractionType::ToggleVisibility,
        action_id: "section-1".to_string(),
        verb: None,
        raw_inputs: json!({}),
        card_instance_id: "card-2".to_string(),
        metadata: json!({ "visible": false }),
    });

    let result = handle_invocation(invocation).expect("toggle");
    assert!(result
        .state_updates
        .iter()
        .any(|op| matches!(op, component_adaptive_card::StateUpdateOp::Set { path, value } if path == "ui.visibility.section-1" && value == &json!(false))));
}

#[test]
fn feature_summary_detects_actions_and_media() {
    let card = json!({
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            { "type": "Media", "sources": [ { "mimeType": "video/mp4", "url": "https://example.com" } ] }
        ],
        "actions": [
            { "type": "Action.ShowCard", "card": { "type": "AdaptiveCard", "version": "1.6", "body": [] } },
            { "type": "Action.ToggleVisibility", "targetElements": ["x"] }
        ]
    });
    let invocation = base_invocation(card);
    let result = handle_invocation(invocation).expect("feature detection");

    assert!(result.card_features.uses_media);
    assert!(result.card_features.uses_show_card);
    assert!(result.card_features.uses_toggle_visibility);
    assert!(
        result
            .card_features
            .used_actions
            .iter()
            .any(|a| a == "Action.ShowCard")
    );
}

#[test]
fn validation_reports_choice_set_and_toggle_rules() {
    let card = json!({
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            { "type": "Input.ChoiceSet", "id": "choices" },
            { "type": "Input.Toggle", "id": "toggle", "title": "" }
        ],
        "actions": [
            { "type": "Action.ToggleVisibility", "targetElements": [] },
            { "type": "Action.ShowCard", "card": "invalid" }
        ]
    });
    let invocation = base_invocation(card);
    let result = handle_invocation(invocation).expect("validation");
    let issues: Vec<String> = result
        .validation_issues
        .iter()
        .map(|v| v.code.clone())
        .collect();
    assert!(issues.iter().any(|c| c == "missing-choices"));
    assert!(issues.iter().any(|c| c == "missing-title"));
    assert!(issues.iter().any(|c| c == "empty-target-elements"));
    assert!(issues.iter().any(|c| c == "invalid-card"));
}

#[test]
fn validation_catches_media_sources() {
    let card = json!({
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            { "type": "Media", "sources": [] }
        ]
    });
    let invocation = base_invocation(card);
    let result = handle_invocation(invocation).expect("validation");
    let codes: Vec<String> = result
        .validation_issues
        .iter()
        .map(|i| i.code.clone())
        .collect();
    assert!(codes.iter().any(|c| c == "missing-sources"));
}

#[test]
fn host_asset_registry_resolves_assets() {
    let _ = register_host_asset_callback(Box::new(|name| {
        if name == "host-card" {
            Some("tests/assets/cards/simple.json".to_string())
        } else {
            None
        }
    }));
    let invocation = AdaptiveCardInvocation {
        card_source: CardSource::Asset,
        card_spec: CardSpec {
            asset_path: Some("host-card".to_string()),
            ..Default::default()
        },
        payload: json!({}),
        session: json!({}),
        state: json!({}),
        interaction: None,
        mode: InvocationMode::RenderAndValidate,
        envelope: None,
    };

    let result = handle_invocation(invocation).expect("host registry");
    let card = result.rendered_card.expect("card should render");
    assert_eq!(card["type"], "AdaptiveCard");
}
