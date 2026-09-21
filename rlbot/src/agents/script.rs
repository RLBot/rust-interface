use rlbot_flat::flat::{
    BallPrediction, ConnectionSettings, CoreMessage, FieldInfo, GamePacket, InitComplete,
    MatchComm, MatchConfiguration, PingResponse, RenderingStatus,
};

use crate::{RLBotConnection, StartingInfo, util::PacketQueue};

use super::AgentError;

#[allow(unused_variables)]
/// The behavior of a script: an interface with no controllables.
///
/// Scripts can't drive cars. They observe the match and manage it: state
/// setting (see [`DesiredGameState`](crate::flat::DesiredGameState)), match
/// communication, and debug rendering. [`run_script_agent`] creates one
/// instance on one thread; your own `agent_id` is handed to [`new`] since
/// there is no controllable to identify you by.
///
/// [`new`]: ScriptAgent::new
pub trait ScriptAgent {
    /// Create the script instance.
    ///
    /// Unlike the other agent kinds there is no controllable, so your
    /// `agent_id` is passed in directly. The `packet_queue` lets you send
    /// packets before the first tick.
    fn new(
        agent_id: String,
        match_configuration: MatchConfiguration,
        field_info: FieldInfo,
        packet_queue: &mut PacketQueue,
    ) -> Self;
    /// React to the latest game state. Called for every [`GamePacket`].
    ///
    /// Queue management packets (state setting, comms, rendering) here.
    ///
    /// [`GamePacket`]: crate::flat::GamePacket
    fn tick(&mut self, game_packet: GamePacket, packet_queue: &mut PacketQueue);
    /// React to a match communication (quick chat / club message).
    ///
    /// Only fires if you passed `wants_comms: true` to [`run_script_agent`].
    fn on_match_comm(&mut self, match_comm: MatchComm, packet_queue: &mut PacketQueue) {}
    /// React to a ball prediction update.
    ///
    /// Only fires if you passed `wants_ball_predictions: true` to
    /// [`run_script_agent`].
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

/// Run a single [`ScriptAgent`] with no controllables on one thread.
/// Ok(()) means a successful exit, i.e. core sent a disconnect.
///
/// # Errors
///
/// Returns an error if the agent panics or the connection fails.
pub fn run_script_agent<T: ScriptAgent>(
    agent_id: String,
    wants_ball_predictions: bool,
    wants_comms: bool,
    mut connection: RLBotConnection,
) -> Result<(), AgentError> {
    connection.send_packet(ConnectionSettings {
        agent_id: agent_id.clone(),
        wants_ball_predictions,
        wants_comms,
        close_between_matches: true,
    })?;

    let StartingInfo {
        controllable_team_info: _,
        match_configuration,
        field_info,
    } = connection.get_starting_info()?;

    let mut outgoing_queue = PacketQueue::default();
    let mut agent = T::new(
        agent_id,
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
