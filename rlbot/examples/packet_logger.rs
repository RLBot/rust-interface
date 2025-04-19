use rlbot::{RLBotConnection, flat::ConnectionSettings, util::AgentEnvironment};

fn main() {
    let AgentEnvironment {
        server_addr,
        agent_id,
    } = AgentEnvironment::from_env();
    let agent_id = agent_id.unwrap_or_else(|| "rlbot/rust-packet-logger".into());

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

    loop {
        let packet = rlbot_connection.recv_packet().unwrap();
        println!("{packet:?}");
    }
}
