use std::{env, mem};

use rlbot_flat::flat::InterfaceMessage;

pub struct AgentEnvironment {
    /// Will fallback to 127.0.0.1:23234
    pub server_addr: String,
    /// No fallback and therefor Option<>
    pub agent_id: Option<String>,
}

impl AgentEnvironment {
    // Reads from environment variables RLBOT_SERVER_ADDR/(RLBOT_SERVER_IP & RLBOT_SERVER_PORT) and RLBOT_AGENT_ID
    #[must_use]
    pub fn from_env() -> Self {
        let server_addr = env::var("RLBOT_SERVER_ADDR").unwrap_or_else(|_| {
            format!(
                "{}:{}",
                env::var("RLBOT_SERVER_IP").unwrap_or_else(|_| "127.0.0.1".into()),
                env::var("RLBOT_SERVER_PORT").unwrap_or_else(|_| "23234".into())
            )
        });

        let agent_id = env::var("RLBOT_AGENT_ID").ok().filter(|s| !s.is_empty());

        Self {
            server_addr,
            agent_id,
        }
    }
}

/// A queue of packets to be sent to RLBotServer
pub struct PacketQueue {
    pub(crate) internal_queue: Vec<InterfaceMessage>,
}

impl Default for PacketQueue {
    fn default() -> Self {
        Self::new(16)
    }
}

impl PacketQueue {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            internal_queue: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, packet: impl Into<InterfaceMessage>) {
        self.internal_queue.push(packet.into());
    }

    pub(crate) fn empty(&mut self) -> Vec<InterfaceMessage> {
        mem::take(&mut self.internal_queue)
    }
}
