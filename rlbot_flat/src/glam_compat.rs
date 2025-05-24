pub use super::flat;
pub use glam;

impl From<flat::Vector3> for glam::Vec3 {
    fn from(value: flat::Vector3) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

impl From<flat::Vector3> for glam::Vec3A {
    fn from(value: flat::Vector3) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

impl From<glam::Vec3> for flat::Vector3 {
    fn from(value: glam::Vec3) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

impl From<glam::Vec3A> for flat::Vector3 {
    fn from(value: glam::Vec3A) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}
