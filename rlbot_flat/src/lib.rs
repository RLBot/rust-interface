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
