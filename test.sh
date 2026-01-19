#!/bin/bash
greentic-component test \
  --wasm target/wasm32-wasip2/release/component_adaptive_card.wasm \
  --manifest ./component.manifest.json \
  --op card --input-json '{
    "card_source": "inline",
    "card_spec": {
      "inline_json": {
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
          { "type": "Input.Text", "id": "comment" }
        ],
        "actions": [
          { "type": "Action.Submit", "title": "Save", "id": "save" }
        ]
      }
    },
    "interaction": {
      "interaction_type": "Submit",
      "action_id": "save",
      "card_instance_id": "card-1",
      "raw_inputs": { "comment": "Hello from state" }
    }
  }' \
  --step --op card --input-json '{
    "card_source": "inline",
    "card_instance_id": "card-1",
    "card_spec": {
      "inline_json": {
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
          { "type": "TextBlock", "text": "Saved: @{state.form_data.comment||\"(none)\"}" }
        ]
      }
    }
  }' \
  --state-dump \
  --pretty
