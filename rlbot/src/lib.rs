// TODO: Update asset links
//! <div style="width: 24rem; padding-bottom: 1rem; display: flex">
#![doc=include_str!("../../assets/RLBotRustLogoText.svg")]
//! </div>
//! <style> svg {height: auto} </style>
//!
//! This is a Rust interface for the [RLBot] v5 socket api. RLBot is a framework
//! for creating offline Rocket League bots. This crate lets you write bots
//! using a simple, safe interface that should feel comfortable to Rust
//! developers.
//!
//! [RLBot]: https://rlbot.org/
//!
//! All of the types in the [`flat`] module are generated from the
//! [flatbuffers spec]. Documentation for these types are carried over from the
//! spec. For further documentation about how RLBot works, see the resources
//! linked on the [RLBot Wiki].
//!
//! [flatbuffers spec]: https://github.com/RLBot/flatbuffers-schema
//! [RLBot Wiki]: https://wiki.rlbot.org/
//!
//! The two main different ways of using this crate follows:
//! - The [`agents`] API - This is a **higher-level** interface. The
//!   [run_x_agent] functions initializes an agent for you. Relevant examples:
//!   [atba_agent, atba_hivemind, high_jump_script].
//! - [`RLBotConnection`] – This is a **lower-level** wrapper around the actual
//!   tcp connection to [core] (RLBotServer). It allows you to use
//!   [`send_packet`] and [`recv_packet`] to manually communicate with RLBot.
//!   For documentation on how to do this, refer to the [socket specification].
//!   Relevant examples: [start_match, stop_match, packet_logger and atba_raw]
//!
//! [run_x_agent]: agents#functions
//! [atba_agent, atba_hivemind, high_jump_script]: https://github.com/RLBot/rust-interface/tree/master/rlbot/examples
//! [`send_packet`]: RLBotConnection::send_packet
//! [`recv_packet`]: RLBotConnection::recv_packet
//! [socket specification]: https://wiki.rlbot.org/v5/framework/sockets-specification/
//! [start_match, stop_match, packet_logger and atba_raw]: https://github.com/RLBot/rust-interface/tree/master/rlbot/examples
//! [core]: https://github.com/RLBot/core
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
pub use rlbot_flat::glam;

pub mod flat {
    //! This module contains all of the types that are generated from the
    //! RLBot flatbuffers schema

    pub use rlbot_flat::RLBOT_FLATBUFFERS_SCHEMA_REV;
    pub use rlbot_flat::flat::*;
}

use flat::*;

#[derive(Error, Debug)]
pub enum PacketParseError {
    #[error("Unpacking flatbuffer failed")]
    InvalidFlatbuffer(#[from] planus::Error),
}

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

pub struct StartingInfo {
    pub controllable_team_info: ControllableTeamInfo,
    pub match_configuration: MatchConfiguration,
    pub field_info: FieldInfo,
}

/// A wrapper around a TCP connection to [core](https://github.com/RLBot/core).
pub struct RLBotConnection {
    pub(crate) stream: TcpStream,
    builder: planus::Builder,
    recv_buf: Box<[u8; u16::MAX as usize]>,
}

impl RLBotConnection {
    pub(crate) fn send_packets_enum(
        &mut self,
        packets: impl Iterator<Item = InterfaceMessage>,
    ) -> Result<(), RLBotError> {
        let to_write = packets
            // convert Packet to Vec<u8> that RLBotServer can understand
            .flat_map(|x| {
                build_packet_payload(GenericMessage::from(x), &mut self.builder)
                    .expect("failed to build packet")
            })
            .collect::<Vec<_>>();

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
    pub fn send_packet(&mut self, packet: impl Into<InterfaceMessage>) -> Result<(), RLBotError> {
        self.send_packet_enum(packet.into())
    }

    /// Receive a [`CoreMessage`] from core.
    pub fn recv_packet(&mut self) -> Result<CoreMessage, RLBotError> {
        let mut buf = [0u8; 2];

        self.stream.read_exact(&mut buf)?;

        let data_len = u16::from_be_bytes(buf);

        let buf = &mut self.recv_buf[0..data_len as usize];

        self.stream.read_exact(buf)?;

        let packet_ref: CorePacketRef =
            CorePacketRef::read_as_root(buf).map_err(PacketParseError::InvalidFlatbuffer)?;
        let packet: CorePacket = packet_ref.try_into().unwrap();

        Ok(packet.message)
    }

    /// Sets the TCP connection to core to be non-blocking.
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), RLBotError> {
        self.stream.set_nonblocking(nonblocking)?;
        Ok(())
    }

    /// Establish a new connection to core
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
