use specs::{Read, ReadStorage, WriteStorage, System};
use crate::{
    world::chunk::Chunk,
    components::{position::PositionComponent, velocity::VelocityComponent}
};

pub struct PhysicsSystem;

impl<'a> System<'a> for PhysicsSystem {
    type SystemData = (Read<'a, Chunk>, ReadStorage<'a, VelocityComponent>, WriteStorage<'a, PositionComponent>);

    fn run(&mut self, (chunk, vel, mut pos): Self::SystemData) {
        use specs::Join;

        for (vel, pos) in (&vel, &mut pos).join() {
            pos.value += vel.value;
            dbg!(pos.value);
        }
    }
}