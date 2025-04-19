#[allow(clippy::all, dead_code)]
pub(crate) mod planus_flat;
pub use planus;
pub use planus_flat::rlbot::flat;

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

#[cfg(feature = "glam")]
pub use glam;

#[cfg(feature = "glam")]
impl From<flat::Vector3> for glam::Vec3 {
    fn from(value: flat::Vector3) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

#[cfg(feature = "glam")]
impl From<flat::Vector3> for glam::Vec3A {
    fn from(value: flat::Vector3) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

#[cfg(feature = "glam")]
impl From<glam::Vec3> for flat::Vector3 {
    fn from(value: glam::Vec3) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

#[cfg(feature = "glam")]
impl From<glam::Vec3A> for flat::Vector3 {
    fn from(value: glam::Vec3A) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}
