use std::f32::consts::PI;

use rlbot::{
    Packet, RLBotConnection,
    flat::{ConnectionSettings, ControllerState, PlayerInput},
    util::AgentEnvironment,
};

fn main() {
    let AgentEnvironment {
        server_addr,
        agent_id,
    } = AgentEnvironment::from_env();
    let agent_id = agent_id.unwrap_or_else(|| "rlbot/rust-example/atba_raw".into());

    let mut rlbot_connection = RLBotConnection::new(&server_addr).expect("connection");

    println!("Connected");

    rlbot_connection
        .send_packet(ConnectionSettings {
            wants_ball_predictions: true,
            wants_comms: true,
            close_between_matches: true,
            agent_id,
        })
        .unwrap();

    let mut packets_to_process = vec![];

    // Wait for ControllableTeamInfo to know which indices we control
    let controllable_team_info = loop {
        let packet = rlbot_connection.recv_packet().unwrap();
        if let Packet::ControllableTeamInfo(x) = packet {
            break x;
        }

        packets_to_process.push(packet);
    };

    assert!(
        controllable_team_info.controllables.len() == 1,
        "The raw atba example code does not support hiveminds, please disable the hivemind field in bot.toml"
    );

    let controllable_info = controllable_team_info
        .controllables
        .first()
        .expect("controllables.len() = 1");

    rlbot_connection.send_packet(Packet::InitComplete).unwrap();

    loop {
        let Packet::GamePacket(game_packet) = packets_to_process
            .pop()
            .unwrap_or_else(|| rlbot_connection.recv_packet().unwrap())
        else {
            continue;
        };

        let Some(ball) = game_packet.balls.first() else {
            continue;
        };
        let target = &ball.physics;

        // We're not in the gtp, skip this tick
        if game_packet.players.len() <= controllable_info.index as usize {
            continue;
        }

        let car = game_packet
            .players
            .get(controllable_info.index as usize)
            .unwrap()
            .physics;

        let bot_to_target_angle = f32::atan2(
            target.location.y - car.location.y,
            target.location.x - car.location.x,
        );

        let mut bot_front_to_target_angle = bot_to_target_angle - car.rotation.yaw;

        bot_front_to_target_angle = (bot_front_to_target_angle + PI).rem_euclid(2. * PI) - PI;

        let mut controller = ControllerState::default();

        if bot_front_to_target_angle > 0. {
            controller.steer = 1.;
        } else {
            controller.steer = -1.;
        }

        controller.throttle = 1.;

        rlbot_connection
            .send_packet(PlayerInput {
                player_index: controllable_info.index,
                controller_state: controller,
            })
            .unwrap();
    }
}
