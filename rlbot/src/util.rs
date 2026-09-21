use std::{env, mem};

use rlbot_flat::flat::InterfaceMessage;

/// How an interface finds core and identifies itself, read from the environment.
///
/// RLBot launches your binary with these variables set; the same variables
/// let you override the connection when testing locally:
///
/// | Variable | Meaning | Fallback |
/// |---|---|---|
/// | `RLBOT_SERVER_ADDR` | Full `ip:port` of core | built from the two below |
/// | `RLBOT_SERVER_IP` | Core's IP | `127.0.0.1` |
/// | `RLBOT_SERVER_PORT` | Core's port | `23234` |
/// | `RLBOT_AGENT_ID` | Your interface's id | `None` (you pick one) |
///
/// # Example
///
/// ```rust
/// use rlbot::util::AgentEnvironment;
///
/// let env = AgentEnvironment::from_env();
/// let agent_id = env.agent_id.unwrap_or_else(|| "my-bot".to_string());
/// // let connection = RLBotConnection::new(&env.server_addr)?;
/// ```
pub struct AgentEnvironment {
    /// Will fallback to 127.0.0.1:23234
    pub server_addr: String,
    /// No fallback and therefor Option<>
    pub agent_id: Option<String>,
}

impl AgentEnvironment {
    /// Read the server address and agent id from the environment.
    ///
    /// See [`AgentEnvironment`] for the variables and their fallbacks.
    /// Never fails: missing variables fall back to local defaults.
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

    /// Queue a packet to be sent to core at the end of the current tick.
    ///
    /// Anything implementing `Into<InterfaceMessage>` works:
    /// [`PlayerInput`](crate::flat::PlayerInput),
    /// [`RenderGroup`](crate::flat::RenderGroup),
    /// [`MatchComm`](crate::flat::MatchComm), and so on. The runner drains
    /// the queue after every `tick`, so nothing is sent until then.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rlbot::flat::PlayerInput;
    /// use rlbot::util::PacketQueue;
    ///
    /// let mut packet_queue = PacketQueue::default();
    /// packet_queue.push(PlayerInput {
    ///     player_index: 0,
    ///     controller_state: Default::default(),
    /// });
    /// ```
    pub fn push(&mut self, packet: impl Into<InterfaceMessage>) {
        self.internal_queue.push(packet.into());
    }

    pub(crate) fn empty(&mut self) -> Vec<InterfaceMessage> {
        mem::take(&mut self.internal_queue)
    }
}
