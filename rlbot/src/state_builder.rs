use rlbot_flat::flat::{
    DesiredBallState, DesiredCarState, DesiredGameState, DesiredMatchInfo, DesiredPhysics, Rotator,
    RotatorPartial, Vector3, Vector3Partial,
};

/// Extension methods for easy construction of [DesiredGameState].
///
/// Example:
/// ```rust
/// use rlbot::state_builder::{DesiredCarStateExt, DesiredGameStateExt, DesiredPhysicsExt};
/// use rlbot::flat::{DesiredGameState, Vector3};
/// let mut dgs = DesiredGameState::default();
/// dgs.mod_car(0, |c| {
///     c.set_location(Vector3::default());
///     c.set_boost(100.);
/// });
/// dgs.mod_balls((0..5).map(|i| (i, |b| {
///     b.set_location_z(0.);
///     b.set_velocity_z(0.);
/// })));
/// ```
pub trait DesiredGameStateExt {
    fn mod_match_info(&mut self, build: impl FnOnce(&mut DesiredMatchInfo));

    fn mod_car(&mut self, index: usize, build: impl FnOnce(&mut DesiredCarState));

    fn mod_cars(&mut self, build: impl IntoIterator<Item = (usize, impl Fn(&mut DesiredCarState))>);

    fn mod_ball(&mut self, index: usize, build: impl FnOnce(&mut DesiredBallState));

    fn mod_balls(
        &mut self,
        build: impl IntoIterator<Item = (usize, impl Fn(&mut DesiredBallState))>,
    );
}

#[allow(dead_code)]
impl DesiredGameStateExt for DesiredGameState {
    /// Modify the desired match info.
    fn mod_match_info(&mut self, build: impl FnOnce(&mut DesiredMatchInfo)) {
        build(self.match_info.get_or_insert_default());
    }

    /// Modify the desired car at the given index.
    fn mod_car(&mut self, index: usize, build: impl FnOnce(&mut DesiredCarState)) {
        if self.car_states.len() <= index {
            self.car_states.resize(index + 1, Default::default());
        }
        build(&mut self.car_states[index]);
    }

    /// Modify all desired cars.
    fn mod_cars(
        &mut self,
        build: impl IntoIterator<Item = (usize, impl Fn(&mut DesiredCarState))>,
    ) {
        for (i, func) in build {
            if self.car_states.len() <= i {
                self.car_states.resize(i + 1, Default::default());
            }
            func(&mut self.car_states[i]);
        }
    }

    /// Modify the desired ball at the given index.
    fn mod_ball(&mut self, index: usize, build: impl FnOnce(&mut DesiredBallState)) {
        if self.ball_states.len() <= index {
            self.ball_states.resize(index + 1, Default::default());
        }
        build(&mut self.ball_states[index]);
    }

    /// Modify all desired balls.
    fn mod_balls(
        &mut self,
        build: impl IntoIterator<Item = (usize, impl Fn(&mut DesiredBallState))>,
    ) {
        for (i, func) in build {
            if self.ball_states.len() <= i {
                self.ball_states.resize(i + 1, Default::default());
            }
            func(&mut self.ball_states[i]);
        }
    }
}

/// Extension methods for easy construction of a [DesiredMatchInfo].
pub trait DesiredMatchInfoExt {
    fn set_gravity_z(&mut self, gravity: f32);
    fn set_game_speed(&mut self, speed: f32);
}

#[allow(dead_code)]
impl DesiredMatchInfoExt for DesiredMatchInfo {
    /// Set the desired world gravity z.
    fn set_gravity_z(&mut self, gravity_z: f32) {
        self.world_gravity_z.get_or_insert_default().val = gravity_z;
    }

    /// Set the desired game speed.
    fn set_game_speed(&mut self, speed: f32) {
        self.game_speed.get_or_insert_default().val = speed;
    }
}

/// Extension methods for easy construction of a [DesiredCarState].
pub trait DesiredCarStateExt {
    fn set_boost(&mut self, amount: f32);
    fn mod_physics(&mut self, build: impl FnOnce(&mut DesiredPhysics));
}

impl DesiredCarStateExt for DesiredCarState {
    /// Set the boost amount of this car.
    fn set_boost(&mut self, amount: f32) {
        self.boost_amount.get_or_insert_default().val = amount;
    }

    /// Modify the physics of this car.
    fn mod_physics(&mut self, build: impl FnOnce(&mut DesiredPhysics)) {
        build(self.physics.get_or_insert_default());
    }
}

/// Extension methods for easy construction of a [DesiredBallState].
pub trait DesiredBallStateExt {
    fn mod_physics(&mut self, build: impl FnOnce(&mut DesiredPhysics));
}

impl DesiredBallStateExt for DesiredBallState {
    /// Modify the physics of this ball.
    fn mod_physics(&mut self, build: impl FnOnce(&mut DesiredPhysics)) {
        build(&mut self.physics)
    }
}

/// Extension methods for easy construction of [DesiredPhysics].
pub trait DesiredPhysicsExt {
    fn set_location(&mut self, loc: impl Into<Vector3>);
    fn set_location_x(&mut self, x: f32);
    fn set_location_y(&mut self, y: f32);
    fn set_location_z(&mut self, z: f32);
    fn set_velocity(&mut self, vel: impl Into<Vector3>);
    fn set_velocity_x(&mut self, x: f32);
    fn set_velocity_y(&mut self, y: f32);
    fn set_velocity_z(&mut self, z: f32);
    fn set_rotation(&mut self, rot: impl Into<Rotator>);
    fn set_rotation_pitch(&mut self, pitch: f32);
    fn set_rotation_yaw(&mut self, yaw: f32);
    fn set_rotation_roll(&mut self, roll: f32);
    fn set_angular_velocity(&mut self, ang_vel: impl Into<Vector3>);
    fn set_angular_velocity_x(&mut self, x: f32);
    fn set_angular_velocity_y(&mut self, y: f32);
    fn set_angular_velocity_z(&mut self, z: f32);
}

macro_rules! physics_path {
    ( $self:ident slf ) => {
        $self
    };
    ( $self:ident physics) => {
        $self.physics
    };
    ( $self:ident optional_physics) => {
        $self.physics.get_or_insert_default()
    };
}

macro_rules! desired_physics_ext {
    ( $t:ty; $p:ident ) => {
        impl DesiredPhysicsExt for $t {
            fn set_location(&mut self, loc: impl Into<Vector3>) {
                let loc: Vector3Partial = loc.into().into();
                physics_path!(self $p).location.replace(loc.into());
            }

            fn set_location_x(&mut self, x: f32) {
                physics_path!(self $p).location.get_or_insert_default().x.get_or_insert_default().val = x;
            }

            fn set_location_y(&mut self, y: f32) {
                physics_path!(self $p).location.get_or_insert_default().y.get_or_insert_default().val = y;
            }

            fn set_location_z(&mut self, z: f32) {
                physics_path!(self $p).location.get_or_insert_default().z.get_or_insert_default().val = z;
            }

            fn set_velocity(&mut self, vel: impl Into<Vector3>) {
                let vel: Vector3Partial = vel.into().into();
                physics_path!(self $p).velocity.replace(vel.into());
            }

            fn set_velocity_x(&mut self, x: f32) {
                physics_path!(self $p).velocity.get_or_insert_default().x.get_or_insert_default().val = x;
            }

            fn set_velocity_y(&mut self, y: f32) {
                physics_path!(self $p).velocity.get_or_insert_default().y.get_or_insert_default().val = y;
            }

            fn set_velocity_z(&mut self, z: f32) {
                physics_path!(self $p).velocity.get_or_insert_default().z.get_or_insert_default().val = z;
            }

            fn set_rotation(&mut self, rot: impl Into<Rotator>) {
                let rot: RotatorPartial = rot.into().into();
                physics_path!(self $p).rotation.replace(rot.into());
            }

            fn set_rotation_pitch(&mut self, pitch: f32) {
                physics_path!(self $p).rotation.get_or_insert_default().pitch.get_or_insert_default().val = pitch;
            }

            fn set_rotation_yaw(&mut self, yaw: f32) {
                physics_path!(self $p).rotation.get_or_insert_default().yaw.get_or_insert_default().val = yaw;
            }

            fn set_rotation_roll(&mut self, roll: f32) {
                physics_path!(self $p).rotation.get_or_insert_default().roll.get_or_insert_default().val = roll;
            }

            fn set_angular_velocity(&mut self, ang_vel: impl Into<Vector3>) {
                let ang_vel: Vector3Partial = ang_vel.into().into();
                physics_path!(self $p).angular_velocity.replace(ang_vel.into());
            }

            fn set_angular_velocity_x(&mut self, x: f32) {
                physics_path!(self $p).angular_velocity.get_or_insert_default().x.get_or_insert_default().val = x;
            }

            fn set_angular_velocity_y(&mut self, y: f32) {
                physics_path!(self $p).angular_velocity.get_or_insert_default().y.get_or_insert_default().val = y;
            }

            fn set_angular_velocity_z(&mut self, z: f32) {
                physics_path!(self $p).angular_velocity.get_or_insert_default().z.get_or_insert_default().val = z;
            }
        }
    };
}

desired_physics_ext!(DesiredPhysics; slf);
desired_physics_ext!(DesiredBallState; physics);
desired_physics_ext!(DesiredCarState; optional_physics);
