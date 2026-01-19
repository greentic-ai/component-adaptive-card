#!/bin/bash
greentic-component test \
  --wasm target/wasm32-wasip2/release/component_adaptive_card.wasm \
  --manifest ./component.manifest.json \
  --op card --input-json '{
    "card_source": "asset",
    "card_spec": {
      "asset_path": "card.json"
    },
    "interaction": {
      "interaction_type": "Submit",
      "action_id": "save",
      "card_instance_id": "card-1",
      "raw_inputs": { "comment": "Hello from state" }
    }
  }' \
  --step --op card --input-json '{
    "card_source": "asset",
    "card_instance_id": "card-1",
    "card_spec": {
      "asset_path": "card2.json"
    }
  }' \
  --state-dump \
  --pretty
