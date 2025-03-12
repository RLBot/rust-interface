use rlbot_flat::flat::{
    BallPrediction, ConnectionSettings, FieldInfo, GamePacket, MatchComm, MatchConfiguration,
};

use crate::{
    Packet, RLBotConnection, StartingInfo,
    agents::{PacketQueue, write_multiple_packets},
};

#[allow(unused_variables)]
pub trait Script {
    fn new(
        agent_id: String,
        match_config: MatchConfiguration,
        field_info: FieldInfo,
        packet_queue: &mut PacketQueue,
    ) -> Self;
    fn on_packet(&mut self, game_packet: GamePacket, packet_queue: &mut PacketQueue);
    fn on_match_comm(&mut self, match_comm: MatchComm, packet_queue: &mut PacketQueue) {}
    fn on_ball_prediction(&mut self, ball_prediction: BallPrediction) {}
}

#[derive(thiserror::Error, Debug)]
pub enum ScriptError {
    #[error("Script panicked")]
    ScriptPanic,
    #[error("RLBot failed")]
    PacketParseError(#[from] crate::RLBotError),
}

pub fn run_script<T: Script>(
    agent_id: String,
    wants_ball_predictions: bool,
    wants_comms: bool,
    mut connection: RLBotConnection,
) -> Result<(), ScriptError> {
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

    connection.send_packet(Packet::InitComplete)?;

    let mut ball_prediction = None;
    let mut game_packet = None;
    'main_loop: loop {
        connection.set_nonblocking(true)?;
        while let Ok(Some(packet)) = connection.try_recv_packet() {
            match packet {
                Packet::None => break 'main_loop,
                Packet::MatchComm(match_comm) => {
                    script.on_match_comm(match_comm, &mut outgoing_queue);
                }
                Packet::BallPrediction(ball_pred) => ball_prediction = Some(ball_pred),
                Packet::GamePacket(gp) => game_packet = Some(gp),
                _ => panic!("Unexpected packet: {:?}", packet),
            }
        }
        connection.set_nonblocking(false)?;

        if let Some(game_packet) = game_packet.take() {
            if let Some(ball_prediction) = ball_prediction.take() {
                script.on_ball_prediction(ball_prediction);
            }

            script.on_packet(game_packet, &mut outgoing_queue);

            write_multiple_packets(&mut connection, outgoing_queue.empty().into_iter())?;
        }
    }

    Ok(())
}
