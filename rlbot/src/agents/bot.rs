use std::{io, sync::Arc, thread};

use mio::Interest;

use crate::{
    RLBotConnection, RLBotError, StartingInfo, flat::*, parse_core_message, pkanal,
    util::PacketQueue,
};

use super::AgentError;

#[allow(unused_variables)]
pub trait BotAgent {
    // TODO: Maybe pass a struct?
    fn new(
        team: u32,
        controllable_info: ControllableInfo,
        match_configuration: Arc<MatchConfiguration>,
        field_info: Arc<FieldInfo>,
        packet_queue: &mut PacketQueue,
    ) -> Self;
    fn tick(&mut self, game_packet: &GamePacket, packet_queue: &mut PacketQueue);
    fn on_match_comm(&mut self, match_comm: &MatchComm, packet_queue: &mut PacketQueue) {}
    fn on_ball_prediction(
        &mut self,
        ball_prediction: &BallPrediction,
        packet_queue: &mut PacketQueue,
    ) {
    }
    fn on_rendering_status(
        &mut self,
        rendering_status: &RenderingStatus,
        packet_queue: &mut PacketQueue,
    ) {
    }
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

    // Main loop, broadcast packet to all of the bots, then wait for all of the outgoing vecs
    let mut events = mio::Events::with_capacity(128);
    let mut read_buf: Vec<u8> = Vec::with_capacity(1024);
    'main: loop {
        poll.poll(&mut events, None)
            .expect("couldn't poll with mio");
        for event in &events {
            match event.token() {
                INCOMING => loop {
                    // Read through the mio-registered handle (not the clone in
                    // `connection.stream`) so that mio re-arms the socket's
                    // readiness event. On Windows, mio only re-delivers
                    // readiness after I/O goes through `try_io`.
                    let would_block = drain_socket(&mut mio_stream, &mut read_buf)
                        .map_err(RLBotError::Connection)?;

                    // Broadcast every complete message currently buffered.
                    loop {
                        if read_buf.len() < 2 {
                            break;
                        }
                        let data_len = u16::from_be_bytes([read_buf[0], read_buf[1]]) as usize;
                        if read_buf.len() < 2 + data_len {
                            break;
                        }
                        let payload = read_buf[2..2 + data_len].to_vec();
                        read_buf.drain(..2 + data_len);

                        let packet = parse_core_message(&payload)?;
                        let packet = Arc::new(packet);

                        for (incoming_sender, _) in &threads {
                            if incoming_sender.send(packet.clone()).is_err() {
                                return Err(AgentError::AgentPanic);
                            }
                        }

                        if matches!(&*packet, CoreMessage::DisconnectSignal(_)) {
                            break 'main;
                        }
                    }

                    if would_block {
                        break;
                    }
                },
                OUTGOING => 'outgoing: loop {
                    let Ok(maybe_msgs) = outgoing_recver.try_recv() else {
                        break 'main;
                    };

                    let Some(p) = maybe_msgs else {
                        break 'outgoing;
                    };

                    let to_write = connection.build_interface_messages(p.into_iter())?;
                    write_all_via_mio(&mut mio_stream, &to_write)
                        .map_err(RLBotError::Connection)?;
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

/// Read any bytes currently available from the non-blocking, mio-registered
/// socket via `mio_stream.try_io`. Doing the I/O through `try_io` on the
/// registered handle is required on Windows so that mio re-arms the socket's
/// readiness event for the next `poll`; reading through a cloned handle
/// instead causes `poll` to never wake again.
///
/// Returns `Ok(true)` when the socket would have blocked (no more data right
/// now), otherwise `Ok(false)`.
fn drain_socket(mio_stream: &mut mio::net::TcpStream, read_buf: &mut Vec<u8>) -> io::Result<bool> {
    let mut scratch = [0u8; 8192];
    let ptr = scratch.as_mut_ptr();
    let cap = scratch.len();

    let res = mio_stream.try_io(|| {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawSocket;
            // SAFETY: `recvfrom` is called with a valid connected socket and a
            // buffer that lives for the duration of the call.
            let n = unsafe {
                libc::recvfrom(
                    mio_stream.as_raw_socket() as usize,
                    ptr as *mut libc::c_char,
                    cap as libc::c_int,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if n < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(n as usize)
        }
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            // SAFETY: `read` is called with a valid fd and a buffer that lives
            // for the duration of the call.
            let n = unsafe { libc::read(mio_stream.as_raw_fd(), ptr as *mut libc::c_void, cap) };
            if n < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(n as usize)
        }
    });

    match res {
        Ok(n) if n > 0 => {
            read_buf.extend_from_slice(&scratch[..n]);
            Ok(false)
        }
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(true),
        Err(e) => Err(e),
    }
}

/// Write all of `data` through the non-blocking, mio-registered socket using
/// `mio_stream.try_io`, so that mio can re-arm writability on Windows.
fn write_all_via_mio(mio_stream: &mut mio::net::TcpStream, data: &[u8]) -> io::Result<()> {
    let mut written = 0;
    while written < data.len() {
        let res = mio_stream.try_io(|| {
            #[cfg(windows)]
            {
                use std::os::windows::io::AsRawSocket;
                // SAFETY: `sendto` is called with a valid connected socket and
                // a buffer range that lives for the duration of the call.
                let n = unsafe {
                    libc::sendto(
                        mio_stream.as_raw_socket() as usize,
                        data.as_ptr().add(written) as *const libc::c_char,
                        (data.len() - written) as libc::c_int,
                        0,
                        std::ptr::null(),
                        0,
                    )
                };
                if n < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(n as usize)
            }
            #[cfg(unix)]
            {
                use std::os::unix::io::AsRawFd;
                // SAFETY: `send` is called with a valid fd and a buffer range
                // that lives for the duration of the call.
                let n = unsafe {
                    libc::send(
                        mio_stream.as_raw_fd(),
                        data.as_ptr().add(written) as *const libc::c_void,
                        data.len() - written,
                        0,
                    )
                };
                if n < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(n as usize)
            }
        });

        match res {
            Ok(n) if n > 0 => written += n,
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "socket closed while writing",
                ));
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                // Socket send buffer is full; retry once it drains.
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
