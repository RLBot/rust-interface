use rlbot_flat::flat::{
    BallPrediction, ConnectionSettings, ControllableTeamInfo, CoreMessage, FieldInfo, GamePacket,
    InitComplete, MatchComm, MatchConfiguration, PingResponse, RenderingStatus,
};

use crate::{RLBotConnection, StartingInfo, util::PacketQueue};

use super::AgentError;

#[allow(unused_variables)]
/// The behavior of a hivemind: one agent instance controlling a whole team.
///
/// Unlike [`BotAgent`], which gets one instance per controllable,
/// [`run_hivemind_agent`] creates a single instance on one thread. You see
/// every car on your team in each [`GamePacket`] and answer with one
/// [`PlayerInput`] per car you want to drive. Use this when your cars need
/// shared state or coordinated decisions.
///
/// Note the difference from [`BotAgent::tick`](crate::agents::BotAgent::tick): the packet is passed by value
/// (`GamePacket`, not `&GamePacket`).
///
/// [`BotAgent`]: super::BotAgent
/// [`GamePacket`]: crate::flat::GamePacket
/// [`PlayerInput`]: crate::flat::PlayerInput
pub trait HivemindAgent {
    /// Create the single hivemind instance for the team.
    ///
    /// Called once when the runner starts. The `packet_queue` lets you send
    /// packets before the first tick.
    fn new(
        controllable_team_info: ControllableTeamInfo,
        match_configuration: MatchConfiguration,
        field_info: FieldInfo,
        packet_queue: &mut PacketQueue,
    ) -> Self;
    /// React to the latest game state. Called for every [`GamePacket`].
    ///
    /// Push one [`PlayerInput`] per car you want to drive that tick.
    ///
    /// [`GamePacket`]: crate::flat::GamePacket
    /// [`PlayerInput`]: crate::flat::PlayerInput
    fn tick(&mut self, game_packet: GamePacket, packet_queue: &mut PacketQueue);
    /// React to a match communication (quick chat / club message).
    ///
    /// Only fires if you passed `wants_comms: true` to
    /// [`run_hivemind_agent`].
    fn on_match_comm(&mut self, match_comm: MatchComm, packet_queue: &mut PacketQueue) {}
    /// React to a ball prediction update.
    ///
    /// Only fires if you passed `wants_ball_predictions: true` to
    /// [`run_hivemind_agent`].
    fn on_ball_prediction(
        &mut self,
        ball_prediction: BallPrediction,
        packet_queue: &mut PacketQueue,
    ) {
    }
    /// React to core acknowledging (or dropping) your rendered groups.
    fn on_rendering_status(
        &mut self,
        rendering_status: RenderingStatus,
        packet_queue: &mut PacketQueue,
    ) {
    }
    /// React to a ping round-trip response. Useful for measuring latency.
    fn on_ping_response(&mut self, ping: PingResponse, packet_queue: &mut PacketQueue) {}
}

/// Run a single [`HivemindAgent`] for a whole team on one thread.
/// Ok(()) means a successful exit, i.e. core sent a disconnect.
///
/// Unlike [`run_bot_agents`](super::run_bot_agents), no per-controllable
/// threads are spawned: `tick` sees every car and you reply with one
/// [`PlayerInput`] per car.
///
/// [`PlayerInput`]: crate::flat::PlayerInput
///
/// # Errors
///
/// Returns an error if the agent panics or the connection fails.
pub fn run_hivemind_agent<T: HivemindAgent>(
    agent_id: String,
    wants_ball_predictions: bool,
    wants_comms: bool,
    mut connection: RLBotConnection,
) -> Result<(), AgentError> {
    connection.send_packet(ConnectionSettings {
        agent_id,
        wants_ball_predictions,
        wants_comms,
        close_between_matches: true,
    })?;

    let StartingInfo {
        controllable_team_info,
        match_configuration,
        field_info,
    } = connection.get_starting_info()?;

    let mut outgoing_queue = PacketQueue::default();
    let mut agent = T::new(
        controllable_team_info,
        match_configuration,
        field_info,
        &mut outgoing_queue,
    );

    outgoing_queue.push(InitComplete {});
    connection.send_packets_enum(outgoing_queue.empty().into_iter())?;

    while let Ok(packet) = connection.recv_packet() {
        match packet {
            CoreMessage::DisconnectSignal(_) => break,
            CoreMessage::GamePacket(x) => {
                agent.tick(*x, &mut outgoing_queue);
            }
            CoreMessage::MatchComm(x) => {
                agent.on_match_comm(*x, &mut outgoing_queue);
            }
            CoreMessage::BallPrediction(x) => {
                agent.on_ball_prediction(*x, &mut outgoing_queue);
            }
            CoreMessage::RenderingStatus(x) => {
                agent.on_rendering_status(*x, &mut outgoing_queue);
            }
            CoreMessage::PingResponse(x) => {
                agent.on_ping_response(*x, &mut outgoing_queue);
            }
            CoreMessage::PingRequest(x) => {
                outgoing_queue.push(PingResponse { cookie: x.cookie });
            }
            CoreMessage::FieldInfo(_)
            | CoreMessage::MatchConfiguration(_)
            | CoreMessage::ControllableTeamInfo(_) => {
                unreachable!("Unexpected packet; should not be able to receive this packet type.")
            }
        }

        connection.send_packets_enum(outgoing_queue.empty().into_iter())?;
    }

    Ok(())
}
