use std::collections::{btree_set, BTreeSet};
use std::iter::FromIterator;

use component_scanner::ComponentScanner;
use generational_index::{
    GenerationalIndex, GenerationalIndexAllocator, GenerationalIndexArray,
    GenerationalIndexArrayIntoIter, GenerationalIndexArrayIter, GenerationalIndexArrayIterMut,
};

/// Uniquely identifies an entity, No allocated Entity will be equal to any other allocated Entity,
/// live or dead.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct Entity(GenerationalIndex);

#[derive(Clone)]
pub struct EntityAllocator(GenerationalIndexAllocator);
pub struct EntityScanner<'a>(usize, &'a GenerationalIndexAllocator);

pub type EntitySet = BTreeSet<Entity>;
pub struct EntitySetScanner<'a>(btree_set::Iter<'a, Entity>, &'a EntityAllocator);

/// A map of entities to values that takes advantage of how entities work to very efficiently map
/// values.  Generally only efficient when storing lots of entities for a long time, as it has
/// storage requirements proportional to the largest Entity index encountered.
#[derive(Clone, Default)]
pub struct EntityIndex<T>(GenerationalIndexArray<T>);
pub struct EntityIndexIter<'a, T: 'a>(GenerationalIndexArrayIter<'a, T>);
pub struct EntityIndexIterMut<'a, T: 'a>(GenerationalIndexArrayIterMut<'a, T>);
pub struct EntityIndexIntoIter<T>(GenerationalIndexArrayIntoIter<T>);

impl Entity {
    #[inline]
    pub fn index(&self) -> usize {
        self.0.index()
    }

    #[inline]
    pub fn generation(&self) -> u64 {
        self.0.generation()
    }
}

impl EntityAllocator {
    pub fn new() -> EntityAllocator {
        EntityAllocator(GenerationalIndexAllocator::new())
    }

    pub fn allocate(&mut self) -> Entity {
        Entity(self.0.allocate())
    }

    pub fn deallocate(&mut self, entity: Entity) -> bool {
        self.0.deallocate(entity.0)
    }

    #[inline]
    pub fn is_live(&self, entity: Entity) -> bool {
        self.0.is_live(entity.0)
    }

    pub fn scan_live(&self) -> EntityScanner {
        EntityScanner(0, &self.0)
    }

    pub fn scan_set<'a>(&'a self, set: &'a EntitySet) -> EntitySetScanner<'a> {
        EntitySetScanner(set.iter(), self)
    }

    pub fn prune_set(&self, set: &mut EntitySet) {
        let mut removed = Vec::new();
        for &e in set.iter() {
            if !self.is_live(e) {
                removed.push(e);
            }
        }

        for r in removed {
            set.remove(&r);
        }
    }
}

impl<'a> ComponentScanner for EntityScanner<'a> {
    type Item = Entity;

    fn scan(&mut self, until: Option<usize>) -> Option<(Self::Item, usize)> {
        if until.is_some() && until.unwrap() > self.0 {
            self.0 = until.unwrap();
        }

        while self.0 < self.1.max_allocated_index() {
            if let Some(gen_index) = self.1.live_at_index(self.0) {
                return Some((Entity(gen_index), self.0));
            } else {
                self.0 += 1;
            }
        }
        None
    }
}

impl<'a> ComponentScanner for EntitySetScanner<'a> {
    type Item = Entity;

    fn scan(&mut self, until: Option<usize>) -> Option<(Entity, usize)> {
        while let Some(&entity) = self.0.next() {
            if until.is_none() || entity.index() >= until.unwrap() {
                if self.1.is_live(entity) {
                    return Some((entity, entity.index()));
                }
            }
        }
        None
    }
}

impl<T> EntityIndex<T> {
    pub fn new() -> EntityIndex<T> {
        EntityIndex(GenerationalIndexArray::new())
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Overwrites the entry with the matching index, returns both the Entity and T that were
    /// replaced, which may be an entity from a past generation.
    pub fn insert(&mut self, entity: Entity, value: T) -> Option<(Entity, T)> {
        self.0
            .insert(entity.0, value)
            .map(|(gen_index, value)| (Entity(gen_index), value))
    }

    /// Only removes if the generation matches.
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        self.0.remove(entity.0)
    }

    pub fn contains_key(&self, entity: Entity) -> bool {
        self.0.contains_key(entity.0)
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.0.get(entity.0)
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.0.get_mut(entity.0)
    }

    pub fn retain<F: FnMut(Entity, &mut T) -> bool>(&mut self, mut f: F) {
        self.0
            .retain(move |gen_index, val| f(Entity(gen_index), val))
    }

    pub fn filter_map<F: FnMut(Entity, T) -> Option<T>>(&mut self, mut f: F) {
        self.0
            .filter_map(move |gen_index, val| f(Entity(gen_index), val))
    }

    pub fn iter<'a>(&'a self) -> EntityIndexIter<'a, T> {
        EntityIndexIter(self.0.iter())
    }

    pub fn iter_mut<'a>(&'a mut self) -> EntityIndexIterMut<'a, T> {
        EntityIndexIterMut(self.0.iter_mut())
    }
}

impl<'a, T: 'a> Iterator for EntityIndexIter<'a, T> {
    type Item = (Entity, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|(gen_index, value)| (Entity(gen_index), value))
    }
}

impl<'a, T: 'a> Iterator for EntityIndexIterMut<'a, T> {
    type Item = (Entity, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|(gen_index, value)| (Entity(gen_index), value))
    }
}

impl<T> Iterator for EntityIndexIntoIter<T> {
    type Item = (Entity, T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|(gen_index, value)| (Entity(gen_index), value))
    }
}

impl<'a, T: 'a> IntoIterator for &'a EntityIndex<T> {
    type Item = (Entity, &'a T);
    type IntoIter = EntityIndexIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: 'a> IntoIterator for &'a mut EntityIndex<T> {
    type Item = (Entity, &'a mut T);
    type IntoIter = EntityIndexIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T> IntoIterator for EntityIndex<T> {
    type Item = (Entity, T);
    type IntoIter = EntityIndexIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        EntityIndexIntoIter(self.0.into_iter())
    }
}

impl<T> FromIterator<(Entity, T)> for EntityIndex<T> {
    fn from_iter<I: IntoIterator<Item = (Entity, T)>>(iter: I) -> EntityIndex<T> {
        let mut map = EntityIndex::new();
        for (entity, value) in iter {
            map.insert(entity, value);
        }
        map
    }
}
