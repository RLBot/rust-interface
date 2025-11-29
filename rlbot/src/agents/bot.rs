use std::{io::ErrorKind, sync::Arc, thread};

use mio::Interest;

use crate::{RLBotConnection, RLBotError, StartingInfo, flat::*, pkanal, util::PacketQueue};

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
    fn on_ping_response(&mut self, packet_queue: &mut PacketQueue) {}
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
    'main: loop {
        poll.poll(&mut events, None)
            .expect("couldn't poll with mio");
        for event in &events {
            match event.token() {
                INCOMING => 'incoming: loop {
                    let packet = match connection.recv_packet() {
                        Ok(x) => x,
                        Err(RLBotError::Connection(e)) if e.kind() == ErrorKind::WouldBlock => {
                            break 'incoming;
                        }
                        Err(e) => Err(e)?,
                    };
                    let packet = Arc::new(packet);

                    for (incoming_sender, _) in &threads {
                        if incoming_sender.send(packet.clone()).is_err() {
                            return Err(AgentError::AgentPanic);
                        }
                    }

                    if matches!(&*packet, CoreMessage::DisconnectSignal(_)) {
                        break 'main;
                    }
                },
                OUTGOING => 'outgoing: loop {
                    let Ok(maybe_msgs) = outgoing_recver.try_recv() else {
                        break 'main;
                    };

                    let Some(p) = maybe_msgs else {
                        break 'outgoing;
                    };

                    connection.send_packets_enum(p.into_iter())?;
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
            CoreMessage::PingResponse(_) => {
                agent.on_ping_response(&mut outgoing_queue);
            }
            CoreMessage::PingRequest(_) => {
                outgoing_queue.push(PingResponse {});
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
