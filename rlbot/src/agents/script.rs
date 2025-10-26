use rlbot_flat::flat::{
    BallPrediction, ConnectionSettings, CoreMessage, FieldInfo, GamePacket, InitComplete,
    MatchComm, MatchConfiguration, RenderingStatus,
};

use crate::{RLBotConnection, StartingInfo, util::PacketQueue};

use super::AgentError;

#[allow(unused_variables)]
pub trait ScriptAgent {
    fn new(
        agent_id: String,
        match_configuration: MatchConfiguration,
        field_info: FieldInfo,
        packet_queue: &mut PacketQueue,
    ) -> Self;
    fn tick(&mut self, game_packet: GamePacket, packet_queue: &mut PacketQueue);
    fn on_match_comm(&mut self, match_comm: MatchComm, packet_queue: &mut PacketQueue) {}
    fn on_ball_prediction(
        &mut self,
        ball_prediction: BallPrediction,
        packet_queue: &mut PacketQueue,
    ) {
    }
    fn on_rendering_status(
        &mut self,
        rendering_status: RenderingStatus,
        packet_queue: &mut PacketQueue,
    ) {
    }
}

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
