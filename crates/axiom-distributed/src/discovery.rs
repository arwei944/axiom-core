//! Minimal node discovery for local testing.

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, watch};
use tokio::time::{interval, Duration};
use tracing::debug;

use crate::cluster::{ClusterError, ClusterView};
use crate::node::{NodeId, NodeInfo, NodeState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub bind_address: String,
    pub seeds: Vec<String>,
    pub gossip_interval_ms: u64,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:0".to_string(),
            seeds: Vec::new(),
            gossip_interval_ms: 1000,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    NodeJoined(NodeInfo),
    NodeLeft(NodeId),
    NodeUpdated(NodeInfo),
}

pub struct NodeDiscovery {
    config: DiscoveryConfig,
    local: NodeInfo,
    view: ClusterView,
    tx: broadcast::Sender<DiscoveryEvent>,
    shutdown: watch::Sender<bool>,
}

impl NodeDiscovery {
    pub fn new(config: DiscoveryConfig, local: NodeInfo) -> Self {
        let (tx, _rx) = broadcast::channel(64);
        let (shutdown, _shutdown_rx) = watch::channel(false);
        let mut view = ClusterView::default();
        view.nodes
            .push((local.node_id, local.clone(), NodeState::Alive));

        Self {
            config,
            local,
            view,
            tx,
            shutdown,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DiscoveryEvent> {
        self.tx.subscribe()
    }

    pub fn local(&self) -> &NodeInfo {
        &self.local
    }

    pub fn view(&self) -> &ClusterView {
        &self.view
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown.send(true);
    }

    pub async fn start(&mut self) -> Result<(), ClusterError> {
        let mut tick = interval(Duration::from_millis(self.config.gossip_interval_ms));
        let mut shutdown_rx = self.shutdown.subscribe();
        let seeds = self.config.seeds.clone();

        loop {
            tokio::select! {
                _ = tick.tick() => {
                    self.gossip_step(&seeds).await;
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        debug!(node = %self.local.node_id, "discovery shutdown");
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn gossip_step(&mut self, seeds: &[String]) {
        for seed in seeds {
            debug!(node = %self.local.node_id, seed = seed, "gossip to seed");
        }
    }
}
