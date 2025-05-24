use std::{
    io::{Read, Write},
    net::{AddrParseError, SocketAddr, TcpStream},
    str::FromStr,
};

use rlbot_flat::planus::{self, ReadAsRoot};
use thiserror::Error;

pub mod agents;
pub mod util;

#[cfg(feature = "glam")]
pub use rlbot_flat::glam;

pub use rlbot_flat::flat;

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

pub struct RLBotConnection {
    stream: TcpStream,
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

    pub fn send_packet(&mut self, packet: impl Into<InterfaceMessage>) -> Result<(), RLBotError> {
        self.send_packet_enum(packet.into())
    }

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

    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), RLBotError> {
        self.stream.set_nonblocking(nonblocking)?;
        Ok(())
    }

    pub fn new(addr: &str) -> Result<Self, RLBotError> {
        let stream = TcpStream::connect(SocketAddr::from_str(addr)?)?;

        stream.set_nodelay(true)?;

        Ok(Self {
            stream,
            builder: planus::Builder::with_capacity(1024),
            recv_buf: Box::new([0u8; u16::MAX as usize]),
        })
    }

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
