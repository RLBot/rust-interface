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
