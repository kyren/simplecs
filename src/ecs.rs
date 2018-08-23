use std::any::TypeId;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use anymap::AnyMap;
use downcast_rs::Downcast;

use component::{Component, ComponentStorage};
use entity::{Entity, EntityAllocator, EntityScanner, EntitySet, EntitySetScanner};

pub struct Ecs {
    entities: EntityAllocator,
    components: HashMap<TypeId, Box<GenericComponentEntry>>,
}

#[derive(Debug, Fail)]
#[fail(display = "ECS component type is unregistered")]
pub struct UnregisteredComponent;

impl Ecs {
    pub fn new() -> Ecs {
        Ecs {
            entities: EntityAllocator::new(),
            components: HashMap::new(),
        }
    }

    pub fn register_component<T: 'static + Component>(&mut self) {
        let type_id = TypeId::of::<T>();
        if !self.components.contains_key(&type_id) {
            self.components
                .insert(type_id, Box::new(ComponentEntry::<T::Storage>::new()));
        }
    }

    pub fn add_entity(
        &mut self,
        components: Option<AnyMap>,
    ) -> Result<Entity, UnregisteredComponent> {
        let entity = self.entities.allocate();
        match components {
            Some(components) => self.insert_components(entity, components).map(|_| entity),
            None => Ok(entity),
        }
    }

    /// If the entity is dead, does nothing and returns None, otherwise returns the set of
    /// overwritten components.
    pub fn insert_components(
        &mut self,
        entity: Entity,
        mut components: AnyMap,
    ) -> Result<Option<AnyMap>, UnregisteredComponent> {
        if !self.entities.is_live(entity) {
            return Ok(None);
        }

        let mut overwritten = AnyMap::new();
        for (_, cm) in self.components.iter_mut() {
            cm.insert_entity_from(entity.index(), &mut components, &mut overwritten);
        }

        // If you could get TypeIds out of anymap::raw::RawMap, you could pre-check this rather than
        // post-check it.
        if !components.is_empty() {
            return Err(UnregisteredComponent);
        }

        Ok(Some(overwritten))
    }

    /// Does nothing and returns None if the entity is already dead.
    pub fn remove_entity(&mut self, entity: Entity) -> Option<AnyMap> {
        if self.entities.deallocate(entity) {
            let mut components = AnyMap::new();
            for (_, cm) in self.components.iter_mut() {
                cm.remove_entity_into(entity.index(), &mut components);
            }
            Some(components)
        } else {
            None
        }
    }

    #[inline]
    pub fn entity_is_live(&self, entity: Entity) -> bool {
        self.entities.is_live(entity)
    }

    /// Clones the entire set of an entity's components
    /// Locks all components for reading
    pub fn clone_entity_components(&self, entity: Entity) -> Option<AnyMap> {
        if self.entities.is_live(entity) {
            let mut components = AnyMap::new();
            for (_, cm) in self.components.iter() {
                cm.clone_entity_into(entity.index(), &mut components);
            }
            Some(components)
        } else {
            None
        }
    }

    /// Scans through all live entities, join this with other component scans to get the Entity
    /// associated with a set of components.
    pub fn scan_entities(&self) -> EntityScanner {
        self.entities.scan_live()
    }

    /// Scans through only the live entries in a given EntitySet.
    pub fn scan_entity_set<'a>(&'a self, set: &'a EntitySet) -> EntitySetScanner<'a> {
        self.entities.scan_set(set)
    }

    /// Remove all the dead entities from the given EntitySet
    pub fn prune_entity_set(&self, set: &mut EntitySet) {
        self.entities.prune_set(set)
    }

    /// Get a read only handle to a component storage by acquiring a read lock on that component
    /// storage.
    pub fn read_component<T>(&self) -> Result<ComponentReadHandle<T>, UnregisteredComponent>
    where
        T: 'static + Component,
    {
        let r = self
            .components
            .get(&TypeId::of::<T>())
            .ok_or(UnregisteredComponent)?
            .downcast_ref::<ComponentEntry<T::Storage>>()
            .expect("improper ComponentEntry type")
            .read();
        Ok(ComponentHandle(r, &self.entities))
    }

    /// Get a read/write handle to a component storage by acquiring a write lock on that component
    /// storage.
    pub fn write_component<T>(&self) -> Result<ComponentWriteHandle<T>, UnregisteredComponent>
    where
        T: 'static + Component,
    {
        let w = self
            .components
            .get(&TypeId::of::<T>())
            .ok_or(UnregisteredComponent)?
            .downcast_ref::<ComponentEntry<T::Storage>>()
            .expect("improper ComponentEntry type")
            .write();
        Ok(ComponentHandle(w, &self.entities))
    }

    /// Get a read/write handle to a component storage by mutable borrow, no locking needs to take
    /// place, but this will borrow Ecs mutably, and thus only one component storage may be obtained
    /// at a time.
    pub fn get_mut_component<T>(
        &mut self,
    ) -> Result<ComponentGetMutHandle<T>, UnregisteredComponent>
    where
        T: Component,
    {
        let w = self
            .components
            .get_mut(&TypeId::of::<T>())
            .ok_or(UnregisteredComponent)?
            .downcast_mut::<ComponentEntry<T::Storage>>()
            .expect("improper ComponentEntry type")
            .get_mut();
        Ok(ComponentHandle(w, &self.entities))
    }
}

impl Clone for Ecs {
    /// For consistency, cloning an Ecs will lock all of the component storages for reading at once,
    /// then clone them, then unlock them.
    fn clone(&self) -> Ecs {
        let mut clone_locks = Vec::new();
        for (type_id, component) in self.components.iter() {
            clone_locks.push((type_id, component.clone_lock()));
        }

        let mut components = HashMap::new();
        for (type_id, clone_lock) in clone_locks {
            components.insert(*type_id, clone_lock());
        }

        Ecs {
            entities: self.entities.clone(),
            components: components,
        }
    }
}

pub struct ComponentHandle<'a, R: 'a>(R, &'a EntityAllocator);

pub type ComponentReadHandle<'a, T> =
    ComponentHandle<'a, RwLockReadGuard<'a, <T as Component>::Storage>>;
pub type ComponentWriteHandle<'a, T> =
    ComponentHandle<'a, RwLockWriteGuard<'a, <T as Component>::Storage>>;
pub type ComponentGetMutHandle<'a, T> = ComponentHandle<'a, &'a mut <T as Component>::Storage>;

impl<'a, 'b, S: ComponentStorage<'b>, R: 'a + Deref<Target = S>> ComponentHandle<'a, R> {
    pub fn get(&'b self, entity: Entity) -> Option<&'b S::Component> {
        self.0.get(entity.index())
    }

    pub fn scan(&'b self) -> S::Scan {
        self.0.scan()
    }
}

pub enum ComponentInsertResult<T> {
    Inserted,
    Updated(T),
    EntityIsDead(T),
}

impl<'a, 'b, S: ComponentStorage<'b>, R: 'a + DerefMut<Target = S>> ComponentHandle<'a, R> {
    pub fn get_mut(&'b mut self, entity: Entity) -> Option<&'b mut S::Component> {
        self.0.get_mut(entity.index())
    }

    pub fn insert(
        &'b mut self,
        entity: Entity,
        component: S::Component,
    ) -> ComponentInsertResult<S::Component> {
        if !self.1.is_live(entity) {
            ComponentInsertResult::EntityIsDead(component)
        } else {
            if let Some(old) = self.0.insert(entity.index(), component) {
                ComponentInsertResult::Updated(old)
            } else {
                ComponentInsertResult::Inserted
            }
        }
    }

    pub fn remove(&'b mut self, entity: Entity) -> Option<S::Component> {
        if self.1.is_live(entity) {
            self.0.remove(entity.index())
        } else {
            None
        }
    }

    pub fn scan_mut(&'b mut self) -> S::ScanMut {
        self.0.scan_mut()
    }
}

struct ComponentEntry<S>(RwLock<S>);

impl<'a, S: 'static + ComponentStorage<'a>> ComponentEntry<S> {
    fn new() -> ComponentEntry<S> {
        ComponentEntry(RwLock::new(S::default()))
    }

    fn read(&self) -> RwLockReadGuard<S> {
        self.0.read().unwrap()
    }

    fn write(&self) -> RwLockWriteGuard<S> {
        self.0.write().unwrap()
    }

    fn get_mut(&mut self) -> &mut S {
        self.0.get_mut().unwrap()
    }
}

trait GenericComponentEntry: Send + Sync + Downcast {
    fn insert_entity_from(
        &mut self,
        entity_index: usize,
        input: &mut AnyMap,
        overwritten: &mut AnyMap,
    );
    fn remove_entity_into(&mut self, entity_index: usize, output: &mut AnyMap);
    fn clone_entity_into(&self, entity_index: usize, output: &mut AnyMap);

    fn clone_lock<'a>(&'a self) -> Box<Fn() -> Box<GenericComponentEntry> + 'a>;
}
impl_downcast!(GenericComponentEntry);

impl<'a, S: 'static + ComponentStorage<'a>> GenericComponentEntry for ComponentEntry<S> {
    fn insert_entity_from(
        &mut self,
        entity_index: usize,
        input: &mut AnyMap,
        overwritten: &mut AnyMap,
    ) {
        if let Some(c) = input.remove::<S::Component>() {
            if let Some(o) = self.0.get_mut().unwrap().insert(entity_index, c) {
                overwritten.insert(o);
            }
        }
    }

    fn remove_entity_into(&mut self, entity_index: usize, output: &mut AnyMap) {
        if let Some(c) = self.0.get_mut().unwrap().remove(entity_index) {
            output.insert(c);
        }
    }

    fn clone_entity_into(&self, entity_index: usize, output: &mut AnyMap) {
        let storage = self.0.read().unwrap();
        if let Some(c) = storage.get(entity_index) {
            output.insert(c.clone());
        }
    }

    fn clone_lock<'b>(&'b self) -> Box<Fn() -> Box<GenericComponentEntry> + 'b> {
        let reader = self.0.read().unwrap();
        Box::new(move || Box::new(ComponentEntry::<S>(RwLock::new(reader.clone()))))
    }
}
