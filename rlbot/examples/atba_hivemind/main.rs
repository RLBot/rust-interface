use std::f32::consts::PI;

use rlbot::{
    RLBotConnection,
    agents::hivemind::{HivemindBotAgent, run_agent},
    flat::{
        ControllableTeamInfo, ControllerState, FieldInfo, GamePacket, MatchConfiguration,
        PlayerInput,
    },
    util::{PacketQueue, RLBotEnvironment},
};

#[allow(dead_code)]
struct AtbaHivemind {
    indices: Vec<u32>,
    spawn_ids: Vec<i32>,
    team: u32,
    names: Vec<String>,
    match_config: MatchConfiguration,
    field_info: FieldInfo,
}

impl HivemindBotAgent for AtbaHivemind {
    fn new(
        controllable_team_info: ControllableTeamInfo,
        match_config: MatchConfiguration,
        field_info: FieldInfo,
        _packet_queue: &mut PacketQueue,
    ) -> Self {
        let names = match_config
            .player_configurations
            .iter()
            .filter_map(|player| {
                controllable_team_info
                    .controllables
                    .iter()
                    .find_map(|controllable| {
                        if controllable.spawn_id == player.spawn_id {
                            Some(player.name.clone())
                        } else {
                            None
                        }
                    })
            })
            .collect();

        let (indices, spawn_ids) = controllable_team_info
            .controllables
            .iter()
            .map(|controllable| (controllable.index, controllable.spawn_id))
            .unzip();

        Self {
            indices,
            spawn_ids,
            team: controllable_team_info.team,
            names,
            match_config,
            field_info,
        }
    }

    fn tick(&mut self, game_packet: GamePacket, packet_queue: &mut PacketQueue) {
        let Some(ball) = game_packet.balls.first() else {
            // If theres no ball, theres nothing to chase, don't do anything
            return;
        };

        // We're not in the gtp, skip this tick
        if game_packet.players.len() <= self.indices[self.indices.len() - 1] as usize {
            return;
        }

        for &index in &self.indices {
            let target = &ball.physics;
            let car = game_packet.players[index as usize].physics;

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

            packet_queue.push(PlayerInput {
                player_index: index,
                controller_state: controller,
            });
        }
    }
}

fn main() {
    let RLBotEnvironment {
        server_addr,
        agent_id,
    } = RLBotEnvironment::from_env();
    let agent_id = agent_id.unwrap_or_else(|| "rlbot/rust-example/atba_hivemind".into());

    println!("Connecting");

    let rlbot_connection = RLBotConnection::new(&server_addr).expect("connection");

    println!("Running!");

    // The hivemind field in your bot.toml file decides if rlbot core is going to
    // start your bot as one or multiple instances of your binary/exe.
    // If the hivemind field is set to true, one instance of your bot will handle
    // all of the bots in a team.

    // Blocking.
    run_agent::<AtbaHivemind>(agent_id.clone(), true, true, rlbot_connection)
        .expect("run_hivemind crashed");

    println!("Hivemind with agent_id `{agent_id}` exited nicely");
}
