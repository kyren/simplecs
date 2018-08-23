#[macro_use]
extern crate failure;
extern crate anymap;
#[macro_use]
extern crate downcast_rs;

pub mod component;
pub mod component_scanner;
pub mod dense_component;
pub mod ecs;
pub mod entity;
pub mod generational_index;
pub mod sparse_component;
pub mod world;
pub mod world_multi_lock;

#[cfg(test)]
mod tests;
