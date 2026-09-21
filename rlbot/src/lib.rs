#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/RLBot/rust-interface/refs/heads/master/assets/RLBotRustLogo.png"
)]

use std::{
    io::{Read, Write},
    net::{AddrParseError, SocketAddr, TcpStream},
    str::FromStr,
};

use rlbot_flat::planus::{self, ReadAsRoot};
use thiserror::Error;

pub mod agents;
mod pkanal;
pub mod render;
pub mod state_builder;
pub mod util;

#[cfg(feature = "glam")]
/// Re-exported [`glam`](https://docs.rs/glam) math types for convenience.
/// Available by default via the `glam` feature; the [`flat`] types convert
/// to and from them.
pub use rlbot_flat::glam;

pub mod flat {
    //! This module contains all of the types that are generated from the
    //! RLBot flatbuffers schema

    pub use rlbot_flat::RLBOT_FLATBUFFERS_SCHEMA_REV;
    pub use rlbot_flat::flat::*;
}

use flat::*;

/// Something went wrong while unpacking a packet received from core.
#[derive(Error, Debug)]
pub enum PacketParseError {
    #[error("Unpacking flatbuffer failed")]
    InvalidFlatbuffer(#[from] planus::Error),
}

/// Something went wrong while talking to core over the socket.
#[derive(Error, Debug)]
pub enum RLBotError {
    #[error("Connection to RLBot failed")]
    Connection(#[from] std::io::Error),
    #[error("Parsing packet failed")]
    PacketParseError(#[from] PacketParseError),
    #[error("Building packet failed")]
    PacketBuildError(#[from] PacketBuildError),
    #[error("Invalid address, cannot parse")]
    InvalidAddrError(#[from] AddrParseError),
}

#[derive(Debug, Clone)]
enum GenericMessage {
    InterfaceMessage(InterfaceMessage),
    CoreMessage(CoreMessage),
}

impl From<InterfaceMessage> for GenericMessage {
    fn from(value: InterfaceMessage) -> Self {
        GenericMessage::InterfaceMessage(value)
    }
}
impl From<CoreMessage> for GenericMessage {
    fn from(value: CoreMessage) -> Self {
        GenericMessage::CoreMessage(value)
    }
}

/// The three packets every interface waits for after connecting.
///
/// Returned by [`RLBotConnection::get_starting_info`]. The agent runners
/// fetch this for you; you only need it when using the connection directly.
pub struct StartingInfo {
    /// Which team and controllables this interface owns.
    pub controllable_team_info: ControllableTeamInfo,
    /// The match setup: players, settings, and mutators.
    pub match_configuration: MatchConfiguration,
    /// Static geometry: goals, boosts, and the ball shape.
    pub field_info: FieldInfo,
}

/// A wrapper around a TCP connection to [core](https://github.com/RLBot/core).
///
/// This is the lower-level API: you send [`InterfaceMessage`]s and receive
/// [`CoreMessage`]s yourself. Most users want the [`agents`] runners instead,
/// which manage this connection for them.
///
/// [`agents`]: crate::agents
pub struct RLBotConnection {
    pub(crate) stream: TcpStream,
    builder: planus::Builder,
    recv_buf: Box<[u8; u16::MAX as usize]>,
}

impl RLBotConnection {
    /// Build the bytes for outgoing packets without touching the socket.
    pub(crate) fn build_interface_messages(
        &mut self,
        packets: impl Iterator<Item = InterfaceMessage>,
    ) -> Vec<u8> {
        packets
            // convert Packet to Vec<u8> that RLBotServer can understand
            .flat_map(|x| {
                build_packet_payload(GenericMessage::from(x), &mut self.builder)
                    .expect("failed to build packet")
            })
            .collect::<Vec<_>>()
    }

    pub(crate) fn send_packets_enum(
        &mut self,
        packets: impl Iterator<Item = InterfaceMessage>,
    ) -> Result<(), RLBotError> {
        let to_write = self.build_interface_messages(packets);

        self.stream.write_all(&to_write)?;
        self.stream.flush()?;

        Ok(())
    }

    fn send_packet_enum(&mut self, packet: InterfaceMessage) -> Result<(), RLBotError> {
        self.stream
            .write_all(&build_packet_payload(packet, &mut self.builder)?)?;
        self.stream.flush()?;
        Ok(())
    }

    /// Send anything that turns into an [`InterfaceMessage`] to core.
    ///
    /// Most packet types convert with `.into()` automatically, e.g.
    /// [`PlayerInput`], [`ConnectionSettings`], and [`RenderGroup`].
    pub fn send_packet(&mut self, packet: impl Into<InterfaceMessage>) -> Result<(), RLBotError> {
        self.send_packet_enum(packet.into())
    }

    /// Receive a [`CoreMessage`] from core.
    ///
    /// Blocks until a full packet arrives. Match on the result to handle
    /// [`GamePacket`]s, [`MatchComm`]s,
    /// and the rest. Use [`set_nonblocking`](Self::set_nonblocking) plus
    /// your own polling if you can't afford to block.
    pub fn recv_packet(&mut self) -> Result<CoreMessage, RLBotError> {
        let mut buf = [0u8; 2];

        self.stream.read_exact(&mut buf)?;

        let data_len = u16::from_be_bytes(buf);

        let buf = &mut self.recv_buf[0..data_len as usize];

        self.stream.read_exact(buf)?;

        parse_core_message(buf)
    }

    /// Sets the TCP connection to core to be non-blocking.
    ///
    /// After this, [`recv_packet`](Self::recv_packet) returns an
    /// [`RLBotError::Connection`] with [`ErrorKind::WouldBlock`] instead of
    /// waiting when no packet has arrived yet.
    ///
    /// [`ErrorKind::WouldBlock`]: std::io::ErrorKind::WouldBlock
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), RLBotError> {
        self.stream.set_nonblocking(nonblocking)?;
        Ok(())
    }

    /// Establish a new connection to core.
    ///
    /// `addr` is usually taken from [`AgentEnvironment::from_env`], e.g.
    /// `"127.0.0.1:23234"` when running locally.
    ///
    /// [`AgentEnvironment::from_env`]: crate::util::AgentEnvironment::from_env
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rlbot::{RLBotConnection, flat::ConnectionSettings};
    ///
    /// let mut connection = RLBotConnection::new("127.0.0.1:23234")?;
    /// connection.send_packet(ConnectionSettings {
    ///     agent_id: "my-interface".to_string(),
    ///     wants_ball_predictions: false,
    ///     wants_comms: false,
    ///     close_between_matches: true,
    /// })?;
    ///
    /// loop {
    ///     let packet = connection.recv_packet()?;
    ///     println!("{packet:?}");
    /// }
    /// # Ok::<(), rlbot::RLBotError>(())
    /// ```
    pub fn new(addr: &str) -> Result<Self, RLBotError> {
        let stream = TcpStream::connect(SocketAddr::from_str(addr)?)?;

        stream.set_nodelay(true)?;

        Ok(Self {
            stream,
            builder: planus::Builder::with_capacity(1024),
            recv_buf: Box::new([0u8; u16::MAX as usize]),
        })
    }

    /// Wait until we get [`ControllableTeamInfo`], [`MatchConfiguration`], and
    /// [`FieldInfo`] from core, discarding all other packets.
    ///
    /// Blocks until all three have arrived. Anything received in the
    /// meantime (e.g. early [`GamePacket`]s) is dropped,
    /// so call this before entering your main loop.
    pub fn get_starting_info(&mut self) -> Result<StartingInfo, RLBotError> {
        let mut controllable_team_info = None;
        let mut match_configuration = None;
        let mut field_info = None;

        loop {
            let packet = self.recv_packet()?;
            match packet {
                CoreMessage::ControllableTeamInfo(x) => controllable_team_info = Some(x),
                CoreMessage::MatchConfiguration(x) => match_configuration = Some(x),
                CoreMessage::FieldInfo(x) => field_info = Some(x),
                _ => {}
            }

            if controllable_team_info.is_some()
                && match_configuration.is_some()
                && field_info.is_some()
            {
                break;
            }
        }

        Ok(StartingInfo {
            controllable_team_info: *controllable_team_info.unwrap(),
            match_configuration: *match_configuration.unwrap(),
            field_info: *field_info.unwrap(),
        })
    }
}

/// Parse a flatbuffer payload into a [`CoreMessage`].
/// The payload excludes the 2-byte length prefix.
pub(crate) fn parse_core_message(buf: &[u8]) -> Result<CoreMessage, RLBotError> {
    let packet_ref: CorePacketRef =
        CorePacketRef::read_as_root(buf).map_err(PacketParseError::InvalidFlatbuffer)?;
    let packet: CorePacket = packet_ref.try_into().unwrap();

    Ok(packet.message)
}

/// Something went wrong while serializing an outgoing packet.
#[derive(Error, Debug)]
pub enum PacketBuildError {
    #[error("Payload too large {0}, couldn't fit in u16")]
    PayloadTooLarge(usize),
}

fn build_packet_payload(
    packet: impl Into<GenericMessage>,
    builder: &mut planus::Builder,
) -> Result<Vec<u8>, PacketBuildError> {
    builder.clear();
    let payload = match packet.into() {
        GenericMessage::InterfaceMessage(x) => {
            let packet: InterfacePacket = x.into();
            builder.finish(packet, None)
        }
        GenericMessage::CoreMessage(x) => {
            let packet: CorePacket = x.into();
            builder.finish(packet, None)
        }
    };
    let data_len_bin = u16::try_from(payload.len())
        .map_err(|_| PacketBuildError::PayloadTooLarge(payload.len()))?
        .to_be_bytes()
        .to_vec();
    Ok([data_len_bin, payload.to_vec()].concat())
}
