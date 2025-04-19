use rlbot::{
    RLBotConnection,
    agents::script::{Script, run_agent},
    flat::{
        DesiredCarState, DesiredGameState, DesiredPhysics, FieldInfo, GamePacket,
        MatchConfiguration, MatchPhase, Vector3Partial,
    },
    util::{PacketQueue, RLBotEnvironment},
};

#[allow(dead_code)]
struct MyScript {
    agent_id: String,
    name: String,
    prev_jumps: Vec<bool>,
}

impl Script for MyScript {
    fn new(
        agent_id: String,
        match_config: MatchConfiguration,
        _field_info: FieldInfo,
        _packet_queue: &mut PacketQueue,
    ) -> Self {
        let name = match_config
            .script_configurations
            .iter()
            .find_map(|script| {
                if script.agent_id == agent_id {
                    Some(script.name.clone())
                } else {
                    None
                }
            })
            .unwrap();

        Self {
            agent_id,
            name,
            prev_jumps: vec![false; match_config.player_configurations.len()],
        }
    }

    fn tick(&mut self, game_packet: GamePacket, packet_queue: &mut PacketQueue) {
        if game_packet.match_info.match_phase != MatchPhase::Active {
            return;
        }

        if self.prev_jumps.len() != game_packet.players.len() {
            self.prev_jumps.resize(game_packet.players.len(), false);
        }

        let mut car_states = Vec::with_capacity(game_packet.players.len());

        for (player, prev_jump) in game_packet.players.iter().zip(self.prev_jumps.iter_mut()) {
            if player.last_input.jump && !*prev_jump {
                let mut physics: Box<DesiredPhysics> = Box::default();
                let mut velocity: Box<Vector3Partial> = Box::default();
                velocity.z = Some(rlbot::flat::Float {
                    // make cars jump super high!
                    val: player.physics.velocity.z + 1000.0,
                });
                physics.velocity = Some(velocity);

                car_states.push(DesiredCarState {
                    physics: Some(physics),
                    ..Default::default()
                });
            } else {
                car_states.push(DesiredCarState::default());
            }

            *prev_jump = player.last_input.jump;
        }

        packet_queue.push(DesiredGameState {
            car_states,
            ..Default::default()
        });
    }
}

fn main() {
    let RLBotEnvironment {
        server_addr,
        agent_id,
    } = RLBotEnvironment::from_env();
    let agent_id = agent_id.unwrap_or_else(|| "rlbot/rust-example/high_jump_script".into());
    let rlbot_connection = RLBotConnection::new(&server_addr).expect("connection");

    // Blocking.
    run_agent::<MyScript>(agent_id.clone(), true, true, rlbot_connection)
        .expect("run_agent crashed");

    println!("Script with agent_id `{agent_id}` exited nicely");
}
