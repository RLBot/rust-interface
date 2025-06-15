use std::env::args;

use rlbot::{
    RLBotConnection,
    flat::{
        CustomBot, DebugRendering, ExistingMatchBehavior, GameMode, Human, MatchConfiguration,
        MatchLengthMutator, MutatorSettings, PlayerClass, PlayerConfiguration,
    },
};

fn main() {
    println!("Connecting");

    let mut rlbot_connection = RLBotConnection::new("127.0.0.1:23234").expect("connection");

    println!("Starting match");

    let mut args = args();

    // Usage: ./start_match 5 your-agent-id
    let bots_to_add = args.nth(1).map_or(1, |x| x.parse().unwrap());
    let agent_id = args
        .next()
        .unwrap_or_else(|| "rlbot/rust-example-bot".into());

    let mut player_configurations = (0..bots_to_add)
        .map(|i| PlayerConfiguration {
            variety: PlayerClass::CustomBot(Box::new(CustomBot {
                name: format!("BOT{i}"),
                root_dir: String::default(),
                run_command: String::default(),
                loadout: None,
                agent_id: agent_id.clone(),
                hivemind: true,
            })),
            team: i % 2,
            player_id: 0, // RLBotServer will set this
        })
        .collect::<Vec<_>>();

    // Also add a human
    player_configurations.push(PlayerConfiguration {
        variety: PlayerClass::Human(Box::new(Human {})),
        team: 1,
        player_id: Default::default(),
    });

    let match_configuration = MatchConfiguration {
        player_configurations,
        game_mode: GameMode::Soccar,
        game_map_upk: "UtopiaStadium_P".into(),
        // mutatorSettings CANNOT be None, otherwise RLBot will crash (this is true for v4, maybe not v5)
        mutators: Some(Box::new(MutatorSettings {
            match_length: MatchLengthMutator::Unlimited,
            ..Default::default()
        })),
        existing_match_behavior: ExistingMatchBehavior::Restart,
        enable_rendering: DebugRendering::OnByDefault,
        enable_state_setting: true,
        ..Default::default()
    };

    rlbot_connection
        .send_packet(match_configuration)
        .expect("start_match");
}
