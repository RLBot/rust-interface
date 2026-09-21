use std::{
    io::{self, Read, Write},
    sync::Arc,
    thread,
};

use mio::Interest;

use crate::{
    RLBotConnection, RLBotError, StartingInfo, flat::*, parse_core_message, pkanal,
    util::PacketQueue,
};

use super::AgentError;

#[allow(unused_variables)]
/// The behavior of a single bot controlling one controllable (usually a car).
///
/// Implement this trait and pass it to [`run_bot_agents`]. The runner spawns
/// one instance per controllable, each on its own thread, and calls [`tick`]
/// for every [`GamePacket`] your car receives. Send inputs back by pushing
/// [`PlayerInput`]s onto the [`PacketQueue`].
///
/// [`tick`]: BotAgent::tick
/// [`GamePacket`]: crate::flat::GamePacket
/// [`PlayerInput`]: crate::flat::PlayerInput
/// [`PacketQueue`]: crate::util::PacketQueue
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use rlbot::agents::BotAgent;
/// use rlbot::flat::{ControllableInfo, FieldInfo, GamePacket, MatchConfiguration, PlayerInput};
/// use rlbot::util::PacketQueue;
///
/// struct ChaseAgent {
///     index: u32,
/// }
///
/// impl BotAgent for ChaseAgent {
///     fn new(
///         _team: u32,
///         controllable_info: ControllableInfo,
///         _match_config: Arc<MatchConfiguration>,
///         _field_info: Arc<FieldInfo>,
///         _packet_queue: &mut PacketQueue,
///     ) -> Self {
///         Self { index: controllable_info.index }
///     }
///
///     fn tick(&mut self, game_packet: &GamePacket, packet_queue: &mut PacketQueue) {
///         // ... decide on a controller state from game_packet ...
///         packet_queue.push(PlayerInput {
///             player_index: self.index,
///             controller_state: Default::default(),
///         });
///     }
/// }
/// ```
pub trait BotAgent {
    /// Create a new agent for one controllable.
    ///
    /// Called once per controllable when the runner starts. The
    /// `packet_queue` lets you send packets (e.g. [`RenderGroup`]s) before
    /// the first tick.
    // TODO: Maybe pass a struct?
    fn new(
        team: u32,
        controllable_info: ControllableInfo,
        match_configuration: Arc<MatchConfiguration>,
        field_info: Arc<FieldInfo>,
        packet_queue: &mut PacketQueue,
    ) -> Self;
    /// React to the latest game state. Called for every [`GamePacket`].
    ///
    /// Push a [`PlayerInput`] onto the queue to drive your car this tick.
    /// Pushing nothing is valid and just coasts.
    ///
    /// [`GamePacket`]: crate::flat::GamePacket
    /// [`PlayerInput`]: crate::flat::PlayerInput
    fn tick(&mut self, game_packet: &GamePacket, packet_queue: &mut PacketQueue);
    /// React to a match communication (quick chat / club message).
    ///
    /// Only fires if you passed `wants_comms: true` to [`run_bot_agents`].
    fn on_match_comm(&mut self, match_comm: &MatchComm, packet_queue: &mut PacketQueue) {}
    /// React to a ball prediction update.
    ///
    /// Only fires if you passed `wants_ball_predictions: true` to
    /// [`run_bot_agents`].
    fn on_ball_prediction(
        &mut self,
        ball_prediction: &BallPrediction,
        packet_queue: &mut PacketQueue,
    ) {
    }
    /// React to core acknowledging (or dropping) your rendered groups.
    fn on_rendering_status(
        &mut self,
        rendering_status: &RenderingStatus,
        packet_queue: &mut PacketQueue,
    ) {
    }
    /// React to a ping round-trip response. Useful for measuring latency.
    fn on_ping_response(&mut self, ping: &PingResponse, packet_queue: &mut PacketQueue) {}
}

/// Run multiple agents with n agents per thread. They share a connection.
/// Ok(()) means a successful exit; one of the bots received a None packet.
///
/// # Errors
///
/// Returns an error if an agent panics or if there is an error with the connection.
///
/// # Panics
///
/// Panics if a thread can't be spawned for each agent.
///
/// # Example
///
/// ```no_run
/// use rlbot::{RLBotConnection, agents::{BotAgent, run_bot_agents}, util::AgentEnvironment};
/// # use std::sync::Arc;
/// # use rlbot::flat::{ControllableInfo, FieldInfo, GamePacket, MatchConfiguration};
/// # use rlbot::util::PacketQueue;
/// # struct MyBot;
/// # impl BotAgent for MyBot {
/// #     fn new(_t: u32, _c: ControllableInfo, _m: Arc<MatchConfiguration>, _f: Arc<FieldInfo>, _q: &mut PacketQueue) -> Self { MyBot }
/// #     fn tick(&mut self, _g: &GamePacket, _q: &mut PacketQueue) {}
/// # }
///
/// let env = AgentEnvironment::from_env();
/// let connection = RLBotConnection::new(&env.server_addr)?;
/// let agent_id = env.agent_id.unwrap_or_else(|| "my-bot".to_string());
///
/// // Blocking: returns when core disconnects or no controllables remain.
/// run_bot_agents::<MyBot>(agent_id, false, false, connection)?;
/// # Ok::<(), rlbot::agents::AgentError>(())
/// ```
pub fn run_bot_agents<T: BotAgent>(
    // TODO: Maybe pass a struct?
    agent_id: String,
    wants_ball_predictions: bool,
    wants_comms: bool,
    mut connection: RLBotConnection,
) -> Result<(), AgentError> {
    connection.send_packet(ConnectionSettings {
        agent_id,
        wants_ball_predictions,
        wants_comms,
        close_between_matches: true,
    })?;

    let StartingInfo {
        controllable_team_info,
        match_configuration,
        field_info,
    } = connection.get_starting_info()?;

    if controllable_team_info.controllables.is_empty() {
        // run no bots? no problem, done
        return Ok(());
    }

    let match_configuration = Arc::new(match_configuration);
    let field_info = Arc::new(field_info);

    let num_threads = controllable_team_info.controllables.len();
    let mut threads = Vec::with_capacity(num_threads);

    connection
        .stream
        .set_nonblocking(true)
        .expect("to set nonblocking");

    let mut mio_stream = mio::net::TcpStream::from_std(
        connection
            .stream
            .try_clone()
            .expect("failed to clone connection stream"),
    );

    let mut poll = mio::Poll::new().expect("couldn't create mio::Poll");

    const INCOMING: mio::Token = mio::Token(0);
    const OUTGOING: mio::Token = mio::Token(1);

    poll.registry()
        .register(&mut mio_stream, INCOMING, Interest::READABLE)
        .expect("couldn't register tcp stream as readable");

    let (outgoing_sender, outgoing_recver) =
        pkanal::unbounded::<Vec<InterfaceMessage>>(poll.registry(), OUTGOING);

    for (i, controllable_info) in controllable_team_info.controllables.into_iter().enumerate() {
        let (incoming_sender, incoming_recver) = kanal::unbounded::<Arc<CoreMessage>>();
        let match_configuration = match_configuration.clone();
        let field_info = field_info.clone();

        let outgoing_sender = outgoing_sender.clone();

        threads.push((
            incoming_sender,
            thread::Builder::new()
                .name(format!(
                    "Agent thread {i} (index {})",
                    controllable_info.index,
                ))
                .spawn(move || {
                    run_bot_agent::<T>(
                        incoming_recver,
                        controllable_team_info.team,
                        controllable_info,
                        match_configuration,
                        field_info,
                        outgoing_sender,
                    );
                })
                .unwrap(),
        ));
    }
    // drop never-again-used copy of outgoing_sender
    // DO NOT REMOVE, otherwise outgoing_recver.recv() will never error
    // which we rely on for clean exiting
    drop(outgoing_sender);

    connection.send_packet(InitComplete {})?;

    // Main loop. Do all socket I/O through `mio_stream`, the handle registered
    // with mio. On Windows, mio re-arms the readiness event only after I/O on
    // the registered handle returns `WouldBlock`.
    let mut events = mio::Events::with_capacity(128);
    let mut read_buf: Vec<u8> = Vec::with_capacity(1024);
    let mut out_buf: Vec<u8> = Vec::new();
    let mut writable_registered = false;

    'main: loop {
        poll.poll(&mut events, None)
            .expect("couldn't poll with mio");
        for event in &events {
            match event.token() {
                INCOMING => {
                    if event.is_writable() && !out_buf.is_empty() {
                        match flush_pending(&mut mio_stream, &mut out_buf) {
                            Ok(true) => {
                                poll.registry()
                                    .reregister(&mut mio_stream, INCOMING, Interest::READABLE)
                                    .expect("couldn't reregister tcp stream");
                                writable_registered = false;
                            }
                            Ok(false) => {}
                            Err(e) => return Err(RLBotError::Connection(e).into()),
                        }
                    }
                    if event.is_readable() {
                        'incoming: loop {
                            match drain_socket(&mut mio_stream, &mut read_buf) {
                                Ok(false) => {}
                                Ok(true) => break 'incoming,
                                Err(e) => return Err(RLBotError::Connection(e).into()),
                            }
                        }

                        // Broadcast each complete packet.
                        'packets: loop {
                            if read_buf.len() < 2 {
                                break 'packets;
                            }
                            let data_len = u16::from_be_bytes([read_buf[0], read_buf[1]]);
                            let frame_len = data_len as usize + 2;
                            if read_buf.len() < frame_len {
                                break 'packets;
                            }
                            let frame: Vec<u8> = read_buf.drain(..frame_len).collect();

                            let packet = Arc::new(parse_core_message(&frame[2..])?);

                            for (incoming_sender, _) in &threads {
                                if incoming_sender.send(packet.clone()).is_err() {
                                    return Err(AgentError::AgentPanic);
                                }
                            }

                            if matches!(&*packet, CoreMessage::DisconnectSignal(_)) {
                                break 'main;
                            }
                        }
                    }
                }
                OUTGOING => 'outgoing: loop {
                    let Ok(maybe_msgs) = outgoing_recver.try_recv() else {
                        break 'main;
                    };

                    let Some(p) = maybe_msgs else {
                        break 'outgoing;
                    };

                    out_buf.extend(connection.build_interface_messages(p.into_iter()));

                    // Send the queued bytes when the socket becomes writable.
                    // Only arm writability when there are bytes to send: a
                    // writable edge on an empty buffer would be consumed
                    // without flushing, and then never re-armed.
                    if !out_buf.is_empty() && !writable_registered {
                        poll.registry()
                            .reregister(
                                &mut mio_stream,
                                INCOMING,
                                Interest::READABLE | Interest::WRITABLE,
                            )
                            .expect("couldn't reregister tcp stream");
                        writable_registered = true;
                    }
                },
                _ => unreachable!(),
            }
        }
    }

    for (_, handle) in threads {
        handle.join().unwrap();
    }

    Ok(())
}

/// Read bytes from core through the mio-registered handle.
///
/// Returns `Ok(true)` when the socket would block.
fn drain_socket(mio_stream: &mut mio::net::TcpStream, read_buf: &mut Vec<u8>) -> io::Result<bool> {
    let mut scratch = [0u8; 8192];
    match mio_stream.read(&mut scratch) {
        Ok(n) if n > 0 => {
            read_buf.extend_from_slice(&scratch[..n]);
            Ok(false)
        }
        // A zero-byte read means core closed the connection. Return an
        // error so the loop does not spin forever.
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "connection to core closed",
        )),
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(true),
        Err(e) => Err(e),
    }
}

/// Write queued bytes to core through the mio-registered handle.
///
/// Returns `Ok(true)` when all bytes are sent.
fn flush_pending(mio_stream: &mut mio::net::TcpStream, out_buf: &mut Vec<u8>) -> io::Result<bool> {
    let mut sent = 0;
    while sent < out_buf.len() {
        match mio_stream.write(&out_buf[sent..]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "socket stopped accepting data",
                ));
            }
            Ok(n) => sent += n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(e) => return Err(e),
        }
    }
    if sent == out_buf.len() {
        out_buf.clear();
        Ok(true)
    } else {
        out_buf.drain(..sent);
        Ok(false)
    }
}

fn run_bot_agent<T: BotAgent>(
    incoming_recver: kanal::Receiver<Arc<CoreMessage>>,
    team: u32,
    controllable_info: ControllableInfo,
    match_configuration: Arc<MatchConfiguration>,
    field_info: Arc<FieldInfo>,
    outgoing_sender: pkanal::Sender<Vec<InterfaceMessage>>,
) {
    let mut outgoing_queue = PacketQueue::default();
    let mut agent = T::new(
        team,
        controllable_info,
        match_configuration,
        field_info,
        &mut outgoing_queue,
    );

    outgoing_sender
        .send(outgoing_queue.empty())
        .expect("Couldn't send outgoing");

    loop {
        let Ok(packet) = incoming_recver.recv() else {
            panic!("channel recv failed")
        };

        match &*packet {
            CoreMessage::DisconnectSignal(_) => break,
            CoreMessage::GamePacket(x) => {
                agent.tick(x, &mut outgoing_queue);
            }
            CoreMessage::MatchComm(x) => {
                agent.on_match_comm(x, &mut outgoing_queue);
            }
            CoreMessage::BallPrediction(x) => {
                agent.on_ball_prediction(x, &mut outgoing_queue);
            }
            CoreMessage::RenderingStatus(x) => {
                agent.on_rendering_status(x, &mut outgoing_queue);
            }
            CoreMessage::PingResponse(x) => {
                agent.on_ping_response(x, &mut outgoing_queue);
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

        if outgoing_queue.internal_queue.is_empty() {
            continue; // Skip waking up main thread.
        }

        outgoing_sender
            .send(outgoing_queue.empty())
            .expect("Couldn't send outgoing");
    }

    drop(incoming_recver);

    // Wake outgoing to check if all outgoing_senders are closed.
    // If so, main thread will exit.
    outgoing_sender.drop_and_wake();
}
