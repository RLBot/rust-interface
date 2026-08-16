use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

use rlbot::{
    RLBotConnection,
    agents::{BotAgent, run_bot_agents},
    flat::{
        ControllableInfo, ControllableTeamInfo, ControllerState, CoreMessage, CorePacket,
        FieldInfo, GamePacket, InterfaceMessage, InterfacePacket, InterfacePacketRef,
        MatchConfiguration, PlayerInput,
    },
    util::PacketQueue,
};
use rlbot_flat::planus::{self, ReadAsRoot};

/// A bot that sends one PlayerInput for every GamePacket it receives.
struct TestBot;

impl BotAgent for TestBot {
    fn new(
        _team: u32,
        _controllable_info: ControllableInfo,
        _match_configuration: Arc<MatchConfiguration>,
        _field_info: Arc<FieldInfo>,
        _packet_queue: &mut PacketQueue,
    ) -> Self {
        TestBot
    }

    fn tick(&mut self, _game_packet: &GamePacket, packet_queue: &mut PacketQueue) {
        packet_queue.push(PlayerInput {
            player_index: 0,
            controller_state: ControllerState {
                throttle: 1.0,
                steer: 0.0,
                pitch: 0.0,
                yaw: 0.0,
                roll: 0.0,
                jump: false,
                boost: false,
                handbrake: false,
                use_item: false,
            },
        });
    }
}

/// Serialize a CoreMessage into its flatbuffer payload.
fn core_payload(msg: CoreMessage) -> Vec<u8> {
    let mut builder = planus::Builder::with_capacity(1024);
    let packet: CorePacket = msg.into();
    builder.finish(packet, None).to_vec()
}

/// Prefix a payload with the 2-byte big-endian length.
fn frame(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 2);
    out.extend_from_slice(&u16::try_from(payload.len()).unwrap().to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Read one complete frame from the stream.
fn read_frame(stream: &mut std::net::TcpStream) -> Result<Vec<u8>, String> {
    let mut len_buf = [0u8; 2];
    stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let len = u16::from_be_bytes(len_buf) as usize;
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).map_err(|e| e.to_string())?;
    Ok(payload)
}

fn parse_interface(payload: &[u8]) -> Result<InterfaceMessage, String> {
    let packet_ref = InterfacePacketRef::read_as_root(payload).map_err(|e| e.to_string())?;
    let packet: InterfacePacket = packet_ref
        .try_into()
        .map_err(|e: planus::Error| e.to_string())?;
    Ok(packet.message)
}

/// A mock of RLBot core. Speaks the socket protocol: each frame is a 2-byte
/// big-endian length followed by a flatbuffer payload.
fn run_mock_core(listener: TcpListener, num_game_packets: usize) -> Result<(), String> {
    let (mut stream, _) = listener.accept().map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| e.to_string())?;

    // The client sends its ConnectionSettings first.
    match parse_interface(&read_frame(&mut stream)?)? {
        InterfaceMessage::ConnectionSettings(_) => {}
        _ => return Err("expected ConnectionSettings".into()),
    }

    // Then it waits for the starting info.
    write_frame(
        &mut stream,
        core_payload(CoreMessage::ControllableTeamInfo(Box::new(
            ControllableTeamInfo {
                team: 0,
                controllables: vec![ControllableInfo {
                    index: 0,
                    identifier: 0,
                }],
            },
        ))),
    )?;
    write_frame(
        &mut stream,
        core_payload(CoreMessage::MatchConfiguration(Box::default())),
    )?;
    write_frame(
        &mut stream,
        core_payload(CoreMessage::FieldInfo(Box::default())),
    )?;

    // The client signals readiness.
    match parse_interface(&read_frame(&mut stream)?)? {
        InterfaceMessage::InitComplete(_) => {}
        _ => return Err("expected InitComplete".into()),
    }

    // Send game packets. Split the first frame to exercise partial reads,
    // and send the last two in a single write to exercise multiple frames
    // per read.
    for _ in 0..num_game_packets.saturating_sub(2) {
        write_frame(
            &mut stream,
            core_payload(CoreMessage::GamePacket(Box::default())),
        )?;
    }
    let mut tail = frame(&core_payload(CoreMessage::GamePacket(Box::default())));
    tail.extend(frame(&core_payload(
        CoreMessage::GamePacket(Box::default()),
    )));
    stream.write_all(&tail).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;

    // The client answers each game packet with one PlayerInput.
    for _ in 0..num_game_packets {
        match parse_interface(&read_frame(&mut stream)?)? {
            InterfaceMessage::PlayerInput(_) => {}
            _ => return Err("expected PlayerInput".into()),
        }
    }

    // Disconnect and expect the client to close the connection.
    write_frame(
        &mut stream,
        core_payload(CoreMessage::DisconnectSignal(Box::default())),
    )?;
    loop {
        match read_frame(&mut stream) {
            Ok(_) => {}
            Err(_) => return Ok(()),
        }
    }
}

fn write_frame(stream: &mut std::net::TcpStream, payload: Vec<u8>) -> Result<(), String> {
    stream
        .write_all(&frame(&payload))
        .map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())
}

#[test]
fn event_loop_round_trip() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();

    let (tx, rx) = mpsc::channel::<(&'static str, Result<(), String>)>();

    let server_tx = tx.clone();
    let server_handle = thread::spawn(move || {
        let _ = server_tx.send(("server", run_mock_core(listener, 3)));
    });

    let client_tx = tx.clone();
    let client_handle = thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            let connection = RLBotConnection::new(&addr.to_string()).map_err(|e| e.to_string())?;
            run_bot_agents::<TestBot>("test-bot".to_string(), false, false, connection)
                .map_err(|e| e.to_string())
        })();
        let _ = client_tx.send(("client", result));
    });

    let mut results = Vec::new();
    while results.len() < 2 {
        let msg = rx
            .recv_timeout(Duration::from_secs(30))
            .expect("event loop hung");
        results.push(msg);
    }

    for (who, result) in results {
        assert!(result.is_ok(), "{who} failed: {result:?}");
    }

    server_handle.join().unwrap();
    client_handle.join().unwrap();
}
