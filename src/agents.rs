use std::{
    collections::VecDeque,
    io::Write,
    mem,
    sync::Arc,
    thread::{self},
};

use swap_buffer_queue::{buffer::VecBuffer, SynchronizedQueue};

use crate::{rlbot::*, Packet, RLBotConnection, RLBotError};

#[allow(unused_variables)]
pub trait Agent {
    fn new(controllable_info: ControllableInfo) -> Self;
    fn tick(&mut self, game_packet: GamePacket, packet_queue: &mut PacketQueue) -> ();
    fn on_field_info(&mut self, field_info: FieldInfo, packet_queue: &mut PacketQueue) -> () {}
    fn on_match_settings(
        &mut self,
        match_settings: MatchSettings,
        packet_queue: &mut PacketQueue,
    ) -> () {
    }
    fn on_match_comm(&mut self, match_comm: MatchComm, packet_queue: &mut PacketQueue) -> () {}
    fn on_ball_prediction(
        &mut self,
        ball_prediction: BallPrediction,
        packet_queue: &mut PacketQueue,
    ) -> () {
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    #[error("Agent panicked")]
    AgentPanic,
    #[error("RLBot failed")]
    PacketParseError(#[from] crate::RLBotError),
}

/// A queue of packets to be sent to RLBotServer
pub struct PacketQueue {
    internal_queue: Vec<Packet>,
}

impl PacketQueue {
    pub fn new() -> Self {
        PacketQueue {
            internal_queue: Vec::with_capacity(16),
        }
    }
    pub fn push(&mut self, packet: Packet) {
        self.internal_queue.push(packet);
    }
    fn empty(&mut self) -> Vec<Packet> {
        mem::take(&mut self.internal_queue)
    }
}

/// Run multiple agents on one thread each. They share a connection.
/// Ok(()) means a successful exit; one of the bots received a None packet.
pub fn run_agents<T: Agent>(
    connection_settings: ConnectionSettings,
    mut connection: RLBotConnection,
) -> Result<(), AgentError> {
    connection.send_packet(connection_settings)?;

    let mut packets_to_process = VecDeque::new();

    // Wait for Controllable(Team)Info to know which indices we control
    let controllable_team_info = loop {
        let packet = connection.recv_packet()?;
        if let Packet::ControllableTeamInfo(x) = packet {
            break x;
        } else {
            packets_to_process.push_back(packet);
            continue;
        }
    };

    let mut threads = vec![];

    let outgoing_queue: Arc<SynchronizedQueue<VecBuffer<Vec<Packet>>>> =
        Arc::new(SynchronizedQueue::with_capacity(
            // Allows 1024 packets per thread, should definitely be enough
            controllable_team_info.controllables.len() * 1024,
        ));
    for (i, controllable_info) in controllable_team_info.controllables.iter().enumerate() {
        let incoming_queue: Arc<SynchronizedQueue<VecBuffer<Packet>>> =
            Arc::new(SynchronizedQueue::with_capacity(1024));
        // let thread_send = queue.clone();
        let controllable_info = controllable_info.clone();

        let outgoing_queue = outgoing_queue.clone();

        threads.push((
            incoming_queue.clone(),
            thread::Builder::new()
                .name(format!(
                    "Agent thread {i} (spawn_id: {} index: {})",
                    controllable_info.spawn_id, controllable_info.index
                ))
                .spawn(move || {
                    let mut bot = T::new(controllable_info);
                    let mut incoming_queue_local = VecDeque::<Packet>::with_capacity(8);
                    let mut outgoing_queue_local = PacketQueue::new();

                    loop {
                        let packet = match incoming_queue.try_dequeue() {
                            Ok(packets) => {
                                let mut iter = packets.into_iter();
                                let first = iter.next().unwrap();
                                incoming_queue_local.append(&mut iter.collect());
                                first
                            }
                            Err(_) => {
                                let Some(packet) = incoming_queue_local.pop_front() else {
                                    continue
                                };
                                if incoming_queue_local.len() >= 8 {
                                    // SKIP QUEUE
                                    println!("WARN! Packet queue too long, skipping packets");
                                    incoming_queue_local.drain(..);
                                }
                                packet
                            }
                        };

                        match packet {
                            Packet::None => break,
                            Packet::GamePacket(x) => bot.tick(x, &mut outgoing_queue_local),
                            Packet::FieldInfo(x) => bot.on_field_info(x, &mut outgoing_queue_local),
                            Packet::MatchSettings(x) => {
                                bot.on_match_settings(x, &mut outgoing_queue_local)
                            }
                            Packet::MatchComm(x) => bot.on_match_comm(x, &mut outgoing_queue_local),
                            Packet::BallPrediction(x) => {
                                bot.on_ball_prediction(x, &mut outgoing_queue_local)
                            }
                            _ => unreachable!() /* The rest of the packets are only client -> server */
                        }

                        outgoing_queue.try_enqueue([outgoing_queue_local.empty()]).expect("Outgoing queue should be empty");
                    }
                    // drop(thread_send);
                    // drop(thread_recv);
                })
                .unwrap(),
        ));
    }
    // // drop never-again-used copy of thread_send
    // // NO NOT REMOVE, otherwise main_recv.recv() will never error
    // // which we rely on for clean exiting
    // drop(thread_send);

    // We only need to send one init complete with the first
    // spawn id even though we may be running multiple bots.
    if controllable_team_info.controllables.is_empty() {
        // run no bots? no problem, done
        return Ok(());
    };

    connection.send_packet(Packet::InitComplete)?;

    // Main loop, broadcast packet to all of the bots, then wait for all of the
    // Rust limited to 32 for now, hopefully fixed in the future though not really a big deal
    let mut to_send: [Vec<Packet>; 32] = Default::default();
    let mut finished_thread_count = 0i64;
    loop {
        let mut maybe_packet = packets_to_process.pop_front();
        if maybe_packet.is_none() && connection.stream.peek(&mut 0u16.to_be_bytes()).is_ok() {
            maybe_packet = Some(connection.recv_packet()?);
        };

        if let Some(packet) = maybe_packet {
            for (thread_process_queue, _) in threads.iter() {
                let Ok(_) = thread_process_queue.try_enqueue([packet.clone()]) else {
                    return Err(AgentError::AgentPanic);
                };
            }
        }

        let threads_len = threads.len() as i64;

        while finished_thread_count < threads_len {
            if let Ok(messages) = outgoing_queue.try_dequeue() {
                for msg in messages {
                    to_send[finished_thread_count as usize] = msg;
                    finished_thread_count += 1
                }
            }
            // if Instant::now().duration_since(start)
            //     // 1/120 of a second processing time - 250µs overhead
            //     > Duration::from_secs_f64(1. / 120. - 250. / 1_000_000.)
            // {
            //     // println!("WARN! At least one thread was too slow to respond, skipping");
            //     break; // Timeout, check next tick instead
            // }
        }
        finished_thread_count = 0;

        if to_send.is_empty() {
            continue; // no need to send nothing
        }

        write_multiple_packets(
            &mut connection,
            mem::take(&mut to_send).into_iter().flatten(),
        )?;
    }
}

fn write_multiple_packets(
    connection: &mut RLBotConnection,
    packets: impl Iterator<Item = Packet>,
) -> Result<(), RLBotError> {
    let to_write = packets
        // convert Packet to Vec<u8> that rlbot can understand
        .map(|x| {
            let data_type_bin = x.data_type().to_be_bytes().to_vec();
            let payload = x.build(&mut connection.builder);
            let data_len_bin = (payload.len() as u16).to_be_bytes().to_vec();

            [data_type_bin, data_len_bin, payload].concat()
        })
        .flatten()
        .collect::<Vec<_>>();

    connection.stream.write_all(&to_write)?;
    connection.stream.flush()?;

    Ok(())
}
