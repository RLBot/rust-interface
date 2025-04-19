use rlbot_flat::flat::{
    BallPrediction, ConnectionSettings, FieldInfo, GamePacket, MatchComm, MatchConfiguration,
};

use crate::{
    Packet, RLBotConnection, StartingInfo,
    util::{PacketQueue, write_multiple_packets},
};

use super::AgentError;

#[allow(unused_variables)]
pub trait Script {
    fn new(
        agent_id: String,
        match_configuration: MatchConfiguration,
        field_info: FieldInfo,
        packet_queue: &mut PacketQueue,
    ) -> Self;
    fn tick(&mut self, game_packet: GamePacket, packet_queue: &mut PacketQueue);
    fn on_match_comm(&mut self, match_comm: MatchComm, packet_queue: &mut PacketQueue) {}
    fn on_ball_prediction(&mut self, ball_prediction: BallPrediction) {}
}

pub fn run_agent<T: Script>(
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
    let mut script = T::new(
        agent_id,
        match_configuration,
        field_info,
        &mut outgoing_queue,
    );

    outgoing_queue.push(Packet::InitComplete);
    write_multiple_packets(&mut connection, outgoing_queue.empty().into_iter())?;

    let mut ball_prediction = None;
    let mut game_packet = None;
    'main_loop: loop {
        connection.set_nonblocking(true)?;
        while let Ok(packet) = connection.recv_packet() {
            match packet {
                Packet::None => break 'main_loop,
                Packet::MatchComm(match_comm) => {
                    script.on_match_comm(match_comm, &mut outgoing_queue);
                }
                Packet::BallPrediction(ball_pred) => ball_prediction = Some(ball_pred),
                Packet::GamePacket(gp) => game_packet = Some(gp),
                _ => panic!("Unexpected packet: {packet:?}"),
            }
        }
        connection.set_nonblocking(false)?;

        if let Some(game_packet) = game_packet.take() {
            if let Some(ball_prediction) = ball_prediction.take() {
                script.on_ball_prediction(ball_prediction);
            }

            script.tick(game_packet, &mut outgoing_queue);

            write_multiple_packets(&mut connection, outgoing_queue.empty().into_iter())?;
        }
    }

    Ok(())
}
