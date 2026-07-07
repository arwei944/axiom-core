use super::AxiomRuntime;
use super::RuntimeHealth;

impl AxiomRuntime {
    pub async fn health(&self) -> RuntimeHealth {
        self.health.read().await.clone()
    }

    pub async fn snapshot_viz(&self) -> Result<serde_json::Value, axiom_kernel::error::AxiomError> {
        let cells = self.cells.read().await;
        let cell_nodes = cells
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id.as_str(),
                    "name": r.id.as_str(),
                    "layer": r.layer.as_str(),
                    "status": if r.cell.is_some() { "running" } else { "mailbox" }
                })
            })
            .collect::<Vec<_>>();

        let mut edges = Vec::new();
        for from in cells.iter() {
            for to in cells.iter() {
                if from.id != to.id && from.layer.can_send_to(to.layer) {
                    edges.push(serde_json::json!({
                        "from": from.id.as_str(),
                        "to": to.id.as_str()
                    }));
                }
            }
        }

        let topology = serde_json::json!({
            "cells": cell_nodes,
            "edges": edges
        });

        let timeline = serde_json::json!({"entries": []});

        let entropy_snapshot = self.governor.snapshot();
        let entropy = serde_json::json!({
            "system_entropy": entropy_snapshot.global.value,
            "cell_entropies": entropy_snapshot.per_cell.iter().map(|(k, v)| (k.clone(), *v)).collect::<Vec<_>>(),
            "status": format!("{:?}", entropy_snapshot.global.level())
        });

        let flow = serde_json::json!({"records": []});

        Ok(serde_json::json!({
            "topology": topology,
            "timeline": timeline,
            "entropy": entropy,
            "flow": flow
        }))
    }
}
