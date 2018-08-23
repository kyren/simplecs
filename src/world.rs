use std::any::TypeId;
use std::sync::{LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard};

use anymap::AnyMap;
use failure::Error;

use component::Component;
use ecs::{ComponentGetMutHandle, ComponentReadHandle, ComponentWriteHandle, Ecs};
use entity::{Entity, EntityScanner, EntitySet, EntitySetScanner};

pub struct World {
    ecs: Ecs,
    resources: AnyMap,
}

impl World {
    pub fn new() -> World {
        World {
            ecs: Ecs::new(),
            resources: AnyMap::new(),
        }
    }

    pub fn insert_resource<T: 'static>(&mut self, resource: T) -> Option<T> {
        self.resources
            .insert(ResourceEntry::new(resource))
            .map(|r| r.into_inner().unwrap())
    }

    pub fn remove_resource<T: 'static>(&mut self) -> Option<T> {
        self.resources
            .remove::<ResourceEntry<T>>()
            .map(|r| r.into_inner().unwrap())
    }

    pub fn read_resource<T: 'static>(&self) -> Result<RwLockReadGuard<T>, Error> {
        let resource = self
            .resources
            .get::<ResourceEntry<T>>()
            .ok_or_else(|| format_err!("No such resource {:?}", TypeId::of::<ResourceEntry<T>>()))?;

        Ok(resource.0.read().unwrap())
    }

    pub fn write_resource<T: 'static>(&self) -> Result<RwLockWriteGuard<T>, Error> {
        let resource = self
            .resources
            .get::<ResourceEntry<T>>()
            .ok_or_else(|| format_err!("No such resource {:?}", TypeId::of::<ResourceEntry<T>>()))?;
        Ok(resource.0.write().unwrap())
    }

    pub fn register_component<T: Component>(&mut self) {
        self.ecs.register_component::<T>();
    }

    pub fn add_entity(&mut self, components: Option<AnyMap>) -> Result<Entity, Error> {
        Ok(self.ecs.add_entity(components)?)
    }

    pub fn insert_components(&mut self, entity: Entity, components: AnyMap) -> Result<(), Error> {
        self.ecs.insert_components(entity, components)?;
        Ok(())
    }

    pub fn remove_entity(&mut self, entity: Entity) -> Option<AnyMap> {
        self.ecs.remove_entity(entity)
    }

    pub fn entity_is_live(&self, entity: Entity) -> bool {
        self.ecs.entity_is_live(entity)
    }

    pub fn clone_entity_components(&self, entity: Entity) -> Option<AnyMap> {
        self.ecs.clone_entity_components(entity)
    }

    pub fn scan_entities(&self) -> EntityScanner {
        self.ecs.scan_entities()
    }

    pub fn scan_entity_set<'a>(&'a self, set: &'a EntitySet) -> EntitySetScanner<'a> {
        self.ecs.scan_entity_set(set)
    }

    pub fn prune_entity_set(&self, set: &mut EntitySet) {
        self.ecs.prune_entity_set(set)
    }

    pub fn read_component<T: Component>(&self) -> Result<ComponentReadHandle<T>, Error> {
        Ok(self.ecs.read_component::<T>()?)
    }

    pub fn write_component<T: Component>(&self) -> Result<ComponentWriteHandle<T>, Error> {
        Ok(self.ecs.write_component::<T>()?)
    }

    pub fn get_mut_component<T: Component>(&mut self) -> Result<ComponentGetMutHandle<T>, Error> {
        Ok(self.ecs.get_mut_component::<T>()?)
    }
}

struct ResourceEntry<T>(RwLock<T>);

impl<T: 'static> ResourceEntry<T> {
    fn new(r: T) -> ResourceEntry<T> {
        ResourceEntry::<T>(RwLock::new(r))
    }

    fn into_inner(self) -> LockResult<T> {
        self.0.into_inner()
    }
}
