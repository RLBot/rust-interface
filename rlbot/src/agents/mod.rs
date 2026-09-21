//! Agent frameworks: batteries-included event loops for the three RLBot
//! interface kinds.
//!
//! Pick the trait that matches what your interface controls:
//!
//! - [`BotAgent`] + [`run_bot_agents`]: one agent instance per controllable
//!   (usually one per car), each ticked on its own thread. This is what most
//!   bots want.
//! - [`HivemindAgent`] + [`run_hivemind_agent`]: a single instance that sees
//!   every car on the team and ticks on one thread. Use this when your cars
//!   need shared state or coordinated decisions.
//! - [`ScriptAgent`] + [`run_script_agent`]: no controllables at all, for
//!   match management, state setting, and rendering scripts.
//!
//! All three runners handle the handshake ([`ConnectionSettings`],
//! [`InitComplete`]), answer [`PingRequest`]s, and exit cleanly on
//! [`DisconnectSignal`]. If you need full control over the socket instead,
//! use [`RLBotConnection`] directly.
//!
//! [`ConnectionSettings`]: crate::flat::ConnectionSettings
//! [`InitComplete`]: crate::flat::InitComplete
//! [`PingRequest`]: crate::flat::PingRequest
//! [`DisconnectSignal`]: crate::flat::DisconnectSignal
//! [`RLBotConnection`]: crate::RLBotConnection
//!
//! See the `atba_agent`, `atba_hivemind`, and `high_jump_script` examples in
//! `rlbot/examples` for complete programs.

mod bot;
mod hivemind;
mod script;

pub use {
    bot::{BotAgent, run_bot_agents},
    hivemind::{HivemindAgent, run_hivemind_agent},
    script::{ScriptAgent, run_script_agent},
};

#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    #[error("Agent panicked")]
    AgentPanic,
    #[error("RLBot failed")]
    PacketParseError(#[from] crate::RLBotError),
}
