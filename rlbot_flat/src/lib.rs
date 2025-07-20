pub(crate) mod planus_flat;
pub use planus;
pub use planus_flat::RLBOT_FLATBUFFERS_SCHEMA_REV;
pub use planus_flat::rlbot::flat;

#[cfg(feature = "glam")]
mod glam_compat;
#[cfg(feature = "glam")]
pub use glam_compat::*;

impl From<f32> for flat::Float {
    fn from(value: f32) -> Self {
        Self { val: value }
    }
}

impl From<flat::Vector3> for flat::Vector3Partial {
    fn from(value: flat::Vector3) -> Self {
        Self {
            x: Some(value.x.into()),
            y: Some(value.y.into()),
            z: Some(value.z.into()),
        }
    }
}

impl From<flat::Rotator> for flat::RotatorPartial {
    fn from(value: flat::Rotator) -> Self {
        Self {
            pitch: Some(flat::Float { val: value.pitch }),
            yaw: Some(flat::Float { val: value.yaw }),
            roll: Some(flat::Float { val: value.roll }),
        }
    }
}

impl From<flat::Physics> for flat::DesiredPhysics {
    fn from(value: flat::Physics) -> Self {
        Self {
            location: Some(Box::new(value.location.into())),
            rotation: Some(Box::new(value.rotation.into())),
            velocity: Some(Box::new(value.velocity.into())),
            angular_velocity: Some(Box::new(value.angular_velocity.into())),
        }
    }
}

impl From<flat::InterfaceMessage> for flat::InterfacePacket {
    fn from(message: flat::InterfaceMessage) -> Self {
        flat::InterfacePacket { message }
    }
}

impl From<flat::CoreMessage> for flat::CorePacket {
    fn from(message: flat::CoreMessage) -> Self {
        flat::CorePacket { message }
    }
}

macro_rules! from_render_message {
    ( $( $t:ident ),* ) => {
        $(
        impl From<flat::$t> for flat::RenderMessage {
            fn from(value: flat::$t) -> Self {
                Self {
                    variety: flat::RenderType::$t(Box::new(value)),
                }
            }
        }
        )*
    };
}

from_render_message!(Line3D, PolyLine3D, String2D, String3D, Rect2D, Rect3D);

impl From<flat::Vector3> for flat::RenderAnchor {
    fn from(value: flat::Vector3) -> Self {
        Self {
            world: value,
            relative: None,
        }
    }
}

impl From<flat::RelativeAnchor> for flat::RenderAnchor {
    fn from(value: flat::RelativeAnchor) -> Self {
        Self {
            world: flat::Vector3::default(),
            relative: Some(value),
        }
    }
}

impl From<flat::CarAnchor> for flat::RelativeAnchor {
    fn from(value: flat::CarAnchor) -> Self {
        flat::RelativeAnchor::CarAnchor(Box::new(value))
    }
}

impl From<flat::BallAnchor> for flat::RelativeAnchor {
    fn from(value: flat::BallAnchor) -> Self {
        flat::RelativeAnchor::BallAnchor(Box::new(value))
    }
}
