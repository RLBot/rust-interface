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
    #[error("Invalid data type: {0}")]
    InvalidDataType(u16),
    #[error("Unpacking flatbuffer failed")]
    InvalidFlatbuffer(#[from] planus::Error),
}

#[derive(Error, Debug)]
pub enum RLBotError {
    #[error("Connection to RLBot failed")]
    Connection(#[from] std::io::Error),
    #[error("Parsing packet failed")]
    PacketParseError(#[from] PacketParseError),
    #[error("Invalid address, cannot parse")]
    InvalidAddrError(#[from] AddrParseError),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Packet {
    None,
    GamePacket(GamePacket),
    FieldInfo(FieldInfo),
    StartCommand(StartCommand),
    MatchConfiguration(MatchConfiguration),
    PlayerInput(PlayerInput),
    DesiredGameState(DesiredGameState),
    RenderGroup(RenderGroup),
    RemoveRenderGroup(RemoveRenderGroup),
    MatchComm(MatchComm),
    BallPrediction(BallPrediction),
    ConnectionSettings(ConnectionSettings),
    StopCommand(StopCommand),
    SetLoadout(SetLoadout),
    InitComplete,
    ControllableTeamInfo(ControllableTeamInfo),
}

macro_rules! gen_impl_from_flat_packet {
    ($($x:ident),+) => {
        $(
            impl From<$x> for Packet {
                fn from(x: $x) -> Self {
                    Packet::$x(x)
                }
            }
        )+
    };
}

gen_impl_from_flat_packet!(
    // None
    GamePacket,
    FieldInfo,
    StartCommand,
    MatchConfiguration,
    PlayerInput,
    DesiredGameState,
    RenderGroup,
    RemoveRenderGroup,
    MatchComm,
    BallPrediction,
    ConnectionSettings,
    StopCommand,
    SetLoadout,
    // InitComplete
    ControllableTeamInfo
);

impl Packet {
    #[must_use]
    pub const fn data_type(&self) -> u16 {
        match *self {
            Self::None => 0,
            Self::GamePacket(_) => 1,
            Self::FieldInfo(_) => 2,
            Self::StartCommand(_) => 3,
            Self::MatchConfiguration(_) => 4,
            Self::PlayerInput(_) => 5,
            Self::DesiredGameState(_) => 6,
            Self::RenderGroup(_) => 7,
            Self::RemoveRenderGroup(_) => 8,
            Self::MatchComm(_) => 9,
            Self::BallPrediction(_) => 10,
            Self::ConnectionSettings(_) => 11,
            Self::StopCommand(_) => 12,
            Self::SetLoadout(_) => 13,
            Self::InitComplete => 14,
            Self::ControllableTeamInfo(_) => 15,
        }
    }

    pub fn build(self, builder: &mut planus::Builder) -> Vec<u8> {
        // TODO: make this mess nicer
        macro_rules! p {
            ($($x:ident),+; $($y:ident),+) => {
                match self {
                    $(
                        Self::$x => Vec::new(),
                    )+
                    $(
                        Self::$y(x) => {
                            builder.clear();
                            builder.finish(x, None).to_vec()
                        },
                    )+
                }
            };
        }

        p!(
            // Empty payload:
            None, InitComplete;
            // Flatbuffer payload:
            GamePacket, FieldInfo, StartCommand, MatchConfiguration, PlayerInput,
            DesiredGameState, RenderGroup, RemoveRenderGroup, MatchComm, BallPrediction,
            ConnectionSettings, StopCommand, SetLoadout, ControllableTeamInfo
        )
    }

    pub fn from_payload(data_type: u16, payload: &[u8]) -> Result<Self, PacketParseError> {
        // TODO: make this mess nicer
        macro_rules! p {
            ($e:ident) => {
                Ok(Self::$e)
            };
            ($e:ident, $x:ident) => {
                Ok(Self::$e($x::read_as_root(payload)?.try_into().unwrap()))
            };
            ($($n:literal, $($x:ident),+);+) => {
                match data_type {
                    $(
                        $n => p!(
                            $($x),+
                        ),
                    )+
                    _ => Err(PacketParseError::InvalidDataType(data_type)),
                }
            };

        }

        p!(
            0, None;
            1, GamePacket, GamePacketRef;
            2, FieldInfo, FieldInfoRef;
            3, StartCommand, StartCommandRef;
            4, MatchConfiguration, MatchConfigurationRef;
            5, PlayerInput, PlayerInputRef;
            6, DesiredGameState, DesiredGameStateRef;
            7, RenderGroup, RenderGroupRef;
            8, RemoveRenderGroup, RemoveRenderGroupRef;
            9, MatchComm, MatchCommRef;
            10, BallPrediction, BallPredictionRef;
            11, ConnectionSettings, ConnectionSettingsRef;
            12, StopCommand, StopCommandRef;
            13, SetLoadout, SetLoadoutRef;
            14, InitComplete;
            15, ControllableTeamInfo, ControllableTeamInfoRef
        )
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
    fn send_packet_enum(&mut self, packet: Packet) -> Result<(), RLBotError> {
        let data_type_bin = packet.data_type().to_be_bytes().to_vec();
        let payload = packet.build(&mut self.builder);
        let data_len_bin = u16::try_from(payload.len())
            .expect("Payload can't be greater than a u16")
            .to_be_bytes()
            .to_vec();

        // Join so we make sure everything gets written in the right order
        let joined = [data_type_bin, data_len_bin, payload].concat();

        self.stream.write_all(&joined)?;
        self.stream.flush()?;
        Ok(())
    }

    pub fn send_packet(&mut self, packet: impl Into<Packet>) -> Result<(), RLBotError> {
        self.send_packet_enum(packet.into())
    }

    pub fn recv_packet(&mut self) -> Result<Packet, RLBotError> {
        let mut buf = [0u8; 4];

        self.stream.read_exact(&mut buf)?;

        let data_type = u16::from_be_bytes([buf[0], buf[1]]);
        let data_len = u16::from_be_bytes([buf[2], buf[3]]);

        let buf = &mut self.recv_buf[0..data_len as usize];

        self.stream.read_exact(buf)?;

        let packet = Packet::from_payload(data_type, buf)?;

        Ok(packet)
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
                Packet::ControllableTeamInfo(x) => controllable_team_info = Some(x),
                Packet::MatchConfiguration(x) => match_configuration = Some(x),
                Packet::FieldInfo(x) => field_info = Some(x),
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
            controllable_team_info: controllable_team_info.unwrap(),
            match_configuration: match_configuration.unwrap(),
            field_info: field_info.unwrap(),
        })
    }
}
