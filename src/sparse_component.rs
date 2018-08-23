use std::collections::btree_map;
use std::collections::BTreeMap;

use component::ComponentStorage;
use component_scanner::ComponentScanner;

#[derive(Clone)]
pub struct SparseComponentStorage<T>(BTreeMap<usize, T>);

pub struct SparseComponentScanner<'a, T: 'a> {
    map: &'a BTreeMap<usize, T>,
    iter: btree_map::Range<'a, usize, T>,
}

pub struct SparseComponentScannerMut<'a, T: 'a>(btree_map::IterMut<'a, usize, T>);

impl<T> Default for SparseComponentStorage<T> {
    fn default() -> SparseComponentStorage<T> {
        SparseComponentStorage(BTreeMap::new())
    }
}

impl<'a, T: 'static + Send + Sync + Clone> ComponentStorage<'a> for SparseComponentStorage<T> {
    type Component = T;
    type Scan = SparseComponentScanner<'a, T>;
    type ScanMut = SparseComponentScannerMut<'a, T>;

    fn get(&self, index: usize) -> Option<&T> {
        self.0.get(&index)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.0.get_mut(&index)
    }

    fn insert(&mut self, index: usize, component: T) -> Option<T> {
        self.0.insert(index, component)
    }

    fn remove(&mut self, index: usize) -> Option<T> {
        self.0.remove(&index)
    }

    fn scan(&'a self) -> Self::Scan {
        SparseComponentScanner {
            map: &self.0,
            iter: self.0.range(..),
        }
    }

    fn scan_mut(&'a mut self) -> Self::ScanMut {
        SparseComponentScannerMut(self.0.iter_mut())
    }
}

impl<T: 'static> SparseComponentStorage<T> {
    pub fn new() -> SparseComponentStorage<T> {
        SparseComponentStorage(BTreeMap::new())
    }
}

impl<'a, T> ComponentScanner for SparseComponentScanner<'a, T> {
    type Item = &'a T;

    fn scan(&mut self, until: Option<usize>) -> Option<(Self::Item, usize)> {
        let until = until.unwrap_or(0);

        if let Some((id, v)) = self.iter.next() {
            if *id >= until {
                return Some((v, *id));
            }
            self.iter = self.map.range(until..);
            if let Some((id, v)) = self.iter.next() {
                return Some((v, *id));
            }
        }

        None
    }
}

impl<'a, T> ComponentScanner for SparseComponentScannerMut<'a, T> {
    type Item = &'a mut T;

    fn scan(&mut self, until: Option<usize>) -> Option<(Self::Item, usize)> {
        // This is very slow, mutably scanning over a sparse component will always unnecessarily
        // scan through the entries one by one
        while let Some((id, v)) = self.0.next() {
            if until.is_none() || *id >= until.unwrap() {
                return Some((v, *id));
            }
        }
        None
    }
}
