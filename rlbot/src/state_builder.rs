use rlbot_flat::flat::{
    DesiredBallState, DesiredCarState, DesiredGameState, DesiredMatchInfo, Rotator, RotatorPartial,
    Vector3, Vector3Partial,
};
use std::ops::Range;

/// Utility for easy construction of [DesiredGameState]s using builder patterns.
///
/// Example:
/// ```example
/// let dgs: DesiredGameState = DesiredStateBuilder::new()
///             .car(0, |c| c
///                 .location(Vector3::default())
///                 .boost(100.))
///             .ball(0, |b| b
///                 .location_z(0.)
///                 .velocity_z(0.)
///             )
///             .build();
/// ```
#[derive(Default, Debug, Clone, PartialEq, PartialOrd)]
pub struct DesiredStateBuilder {
    state: DesiredGameState,
}

#[allow(dead_code)]
impl DesiredStateBuilder {
    pub fn new() -> Self {
        Self {
            state: DesiredGameState::default(),
        }
    }

    /// Modify the desired match info.
    pub fn match_info(
        mut self,
        build: impl FnOnce(DesiredMatchInfoBuilder) -> DesiredMatchInfoBuilder,
    ) -> Self {
        build(DesiredMatchInfoBuilder::new(
            &mut self.state.match_info.get_or_insert_default(),
        ));
        self
    }

    /// Modify the desired ball at the given index.
    pub fn ball(
        mut self,
        index: usize,
        build: impl FnOnce(DesiredBallBuilder) -> DesiredBallBuilder,
    ) -> Self {
        while self.state.ball_states.len() <= index {
            self.state.ball_states.push(Default::default());
        }
        build(DesiredBallBuilder::new(&mut self.state.ball_states[index]));
        self
    }

    /// Modify all desired balls.
    pub fn all_balls(
        mut self,
        range: Range<usize>,
        build: impl Fn(usize, DesiredBallBuilder) -> DesiredBallBuilder,
    ) -> Self {
        while self.state.ball_states.len() < range.end {
            self.state.ball_states.push(Default::default());
        }
        for (i, ball) in self.state.ball_states[range].iter_mut().enumerate() {
            build(i, DesiredBallBuilder::new(ball));
        }
        self
    }

    /// Modify the desired car at the given index.
    pub fn car(
        mut self,
        index: usize,
        build: impl FnOnce(DesiredCarBuilder) -> DesiredCarBuilder,
    ) -> Self {
        while self.state.car_states.len() <= index {
            self.state.car_states.push(Default::default());
        }
        build(DesiredCarBuilder::new(&mut self.state.car_states[index]));
        self
    }

    /// Modify all desired cars.
    pub fn all_cars(
        mut self,
        range: Range<usize>,
        build: impl Fn(usize, DesiredCarBuilder) -> DesiredCarBuilder,
    ) -> Self {
        while self.state.ball_states.len() < range.end {
            self.state.ball_states.push(Default::default());
        }
        for (i, car) in self.state.car_states[range].iter_mut().enumerate() {
            build(i, DesiredCarBuilder::new(car));
        }
        self
    }

    /// Extract the resulting [DesiredGameState].
    pub fn build(self) -> DesiredGameState {
        self.state
    }
}

/// Allows for easy modification of a [DesiredMatchInfo]. See [DesiredStateBuilder].
#[derive(Debug, PartialEq, PartialOrd)]
pub struct DesiredMatchInfoBuilder<'a> {
    info: &'a mut DesiredMatchInfo,
}

#[allow(dead_code)]
impl<'a> DesiredMatchInfoBuilder<'a> {
    pub fn new(info: &'a mut DesiredMatchInfo) -> Self {
        Self { info }
    }

    /// Set the desired world gravity z.
    pub fn gravity_z(self, gravity_z: f32) -> Self {
        self.info.world_gravity_z.get_or_insert_default().val = gravity_z;
        self
    }

    /// Set the desired game speed.
    pub fn game_speed(self, game_speed: f32) -> Self {
        self.info.game_speed.get_or_insert_default().val = game_speed;
        self
    }
}

/// Allows for easy modification of a [DesiredCarState]. See [DesiredStateBuilder].
#[derive(Debug, PartialEq, PartialOrd)]
pub struct DesiredCarBuilder<'a> {
    car: &'a mut DesiredCarState,
}

#[allow(dead_code)]
impl<'a> DesiredCarBuilder<'a> {
    pub fn new(car: &'a mut DesiredCarState) -> Self {
        Self { car }
    }

    /// Set the boost amount of the desired car.
    pub fn boost(self, amount: f32) -> Self {
        self.car.boost_amount = Some(amount.into());
        self
    }

    /// Set the location of the desired car.
    pub fn location(self, loc: impl Into<Vector3>) -> Self {
        let loc: Vector3Partial = loc.into().into();
        self.car
            .physics
            .get_or_insert_default()
            .location
            .replace(loc.into());
        self
    }

    /// Set the location x value of the desired car.
    pub fn location_x(self, x: f32) -> Self {
        self.car
            .physics
            .get_or_insert_default()
            .location
            .get_or_insert_default()
            .x
            .get_or_insert_default()
            .val = x;
        self
    }

    /// Set the location y value of the desired car.
    pub fn location_y(self, y: f32) -> Self {
        self.car
            .physics
            .get_or_insert_default()
            .location
            .get_or_insert_default()
            .y
            .get_or_insert_default()
            .val = y;
        self
    }

    /// Set the location z value of the desired car.
    pub fn location_z(self, z: f32) -> Self {
        self.car
            .physics
            .get_or_insert_default()
            .location
            .get_or_insert_default()
            .z
            .get_or_insert_default()
            .val = z;
        self
    }

    /// Set the velocity of the desired car.
    pub fn velocity(self, vel: impl Into<Vector3>) -> Self {
        let vel: Vector3Partial = vel.into().into();
        self.car
            .physics
            .get_or_insert_default()
            .velocity
            .replace(vel.into());
        self
    }

    /// Set the velocity x value of the desired car.
    pub fn velocity_x(self, x: f32) -> Self {
        self.car
            .physics
            .get_or_insert_default()
            .velocity
            .get_or_insert_default()
            .x
            .get_or_insert_default()
            .val = x;
        self
    }

    /// Set the velocity y value of the desired car.
    pub fn velocity_y(self, y: f32) -> Self {
        self.car
            .physics
            .get_or_insert_default()
            .velocity
            .get_or_insert_default()
            .y
            .get_or_insert_default()
            .val = y;
        self
    }

    /// Set the velocity z value of the desired car.
    pub fn velocity_z(self, z: f32) -> Self {
        self.car
            .physics
            .get_or_insert_default()
            .velocity
            .get_or_insert_default()
            .z
            .get_or_insert_default()
            .val = z;
        self
    }

    /// Set the rotation of the desired car.
    pub fn rotation(self, rot: impl Into<Rotator>) -> Self {
        let rot: RotatorPartial = rot.into().into();
        self.car
            .physics
            .get_or_insert_default()
            .rotation
            .replace(rot.into());
        self
    }

    /// Set the rotation pitch of the desired car.
    pub fn rotation_pitch(self, pitch: f32) -> Self {
        self.car
            .physics
            .get_or_insert_default()
            .rotation
            .get_or_insert_default()
            .pitch
            .get_or_insert_default()
            .val = pitch;
        self
    }

    /// Set the rotation yaw of the desired car.
    pub fn rotation_yaw(self, yaw: f32) -> Self {
        self.car
            .physics
            .get_or_insert_default()
            .rotation
            .get_or_insert_default()
            .yaw
            .get_or_insert_default()
            .val = yaw;
        self
    }

    /// Set the rotation roll of the desired car.
    pub fn rotation_roll(self, roll: f32) -> Self {
        self.car
            .physics
            .get_or_insert_default()
            .rotation
            .get_or_insert_default()
            .roll
            .get_or_insert_default()
            .val = roll;
        self
    }

    /// Set the angular velocity of the desired car.
    pub fn angular_velocity(self, ang_vel: impl Into<Vector3>) -> Self {
        let ang_vel: Vector3Partial = ang_vel.into().into();
        self.car
            .physics
            .get_or_insert_default()
            .angular_velocity
            .replace(ang_vel.into());
        self
    }

    /// Set the angular velocity x value of the desired car.
    pub fn angular_velocity_x(self, x: f32) -> Self {
        self.car
            .physics
            .get_or_insert_default()
            .angular_velocity
            .get_or_insert_default()
            .x
            .get_or_insert_default()
            .val = x;
        self
    }

    /// Set the angular velocity y value of the desired car.
    pub fn angular_velocity_y(self, y: f32) -> Self {
        self.car
            .physics
            .get_or_insert_default()
            .angular_velocity
            .get_or_insert_default()
            .y
            .get_or_insert_default()
            .val = y;
        self
    }

    /// Set the angular velocity z value of the desired car.
    pub fn angular_velocity_z(self, z: f32) -> Self {
        self.car
            .physics
            .get_or_insert_default()
            .angular_velocity
            .get_or_insert_default()
            .z
            .get_or_insert_default()
            .val = z;
        self
    }
}

/// Allows for easy modification of a [DesiredBallState]. See [DesiredStateBuilder].
#[derive(Debug, PartialEq, PartialOrd)]
pub struct DesiredBallBuilder<'a> {
    ball: &'a mut DesiredBallState,
}

#[allow(dead_code)]
impl<'a> DesiredBallBuilder<'a> {
    pub fn new(ball: &'a mut DesiredBallState) -> Self {
        Self { ball }
    }

    /// Set the location of the desired ball.
    pub fn location(self, loc: impl Into<Vector3>) -> Self {
        let loc: Vector3Partial = loc.into().into();
        self.ball.physics.location.replace(loc.into());
        self
    }

    /// Set the location x value of the desired ball.
    pub fn location_x(self, x: f32) -> Self {
        self.ball
            .physics
            .location
            .get_or_insert_default()
            .x
            .get_or_insert_default()
            .val = x;
        self
    }

    /// Set the location y value of the desired ball.
    pub fn location_y(self, y: f32) -> Self {
        self.ball
            .physics
            .location
            .get_or_insert_default()
            .y
            .get_or_insert_default()
            .val = y;
        self
    }

    /// Set the location z value of the desired ball.
    pub fn location_z(self, z: f32) -> Self {
        self.ball
            .physics
            .location
            .get_or_insert_default()
            .z
            .get_or_insert_default()
            .val = z;
        self
    }

    /// Set the velocity of the desired ball.
    pub fn velocity(self, vel: impl Into<Vector3>) -> Self {
        let vel: Vector3Partial = vel.into().into();
        self.ball.physics.velocity.replace(vel.into());
        self
    }

    /// Set the velocity x value of the desired ball.
    pub fn velocity_x(self, x: f32) -> Self {
        self.ball
            .physics
            .velocity
            .get_or_insert_default()
            .x
            .get_or_insert_default()
            .val = x;
        self
    }

    /// Set the velocity y value of the desired ball.
    pub fn velocity_y(self, y: f32) -> Self {
        self.ball
            .physics
            .velocity
            .get_or_insert_default()
            .y
            .get_or_insert_default()
            .val = y;
        self
    }

    /// Set the velocity z value of the desired ball.
    pub fn velocity_z(self, z: f32) -> Self {
        self.ball
            .physics
            .velocity
            .get_or_insert_default()
            .z
            .get_or_insert_default()
            .val = z;
        self
    }

    /// Set the rotation of the desired ball.
    pub fn rotation(self, rot: impl Into<Rotator>) -> Self {
        let rot: RotatorPartial = rot.into().into();
        self.ball.physics.rotation.replace(rot.into());
        self
    }

    /// Set the rotation pitch of the desired ball.
    pub fn rotation_pitch(self, pitch: f32) -> Self {
        self.ball
            .physics
            .rotation
            .get_or_insert_default()
            .pitch
            .get_or_insert_default()
            .val = pitch;
        self
    }

    /// Set the rotation yaw of the desired ball.
    pub fn rotation_yaw(self, yaw: f32) -> Self {
        self.ball
            .physics
            .rotation
            .get_or_insert_default()
            .yaw
            .get_or_insert_default()
            .val = yaw;
        self
    }

    /// Set the rotation roll of the desired ball.
    pub fn rotation_roll(self, roll: f32) -> Self {
        self.ball
            .physics
            .rotation
            .get_or_insert_default()
            .roll
            .get_or_insert_default()
            .val = roll;
        self
    }

    /// Set the angular velocity of the desired ball.
    pub fn angular_velocity(self, ang_vel: impl Into<Vector3>) -> Self {
        let ang_vel: Vector3Partial = ang_vel.into().into();
        self.ball.physics.angular_velocity.replace(ang_vel.into());
        self
    }

    /// Set the angular velocity x value of the desired ball.
    pub fn angular_velocity_x(self, x: f32) -> Self {
        self.ball
            .physics
            .angular_velocity
            .get_or_insert_default()
            .x
            .get_or_insert_default()
            .val = x;
        self
    }

    /// Set the angular velocity y value of the desired ball.
    pub fn angular_velocity_y(self, y: f32) -> Self {
        self.ball
            .physics
            .angular_velocity
            .get_or_insert_default()
            .y
            .get_or_insert_default()
            .val = y;
        self
    }

    /// Set the angular velocity z value of the desired ball.
    pub fn angular_velocity_z(self, z: f32) -> Self {
        self.ball
            .physics
            .angular_velocity
            .get_or_insert_default()
            .z
            .get_or_insert_default()
            .val = z;
        self
    }
}
