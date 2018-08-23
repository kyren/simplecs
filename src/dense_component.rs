use std::cmp;
use std::slice;

use component::ComponentStorage;
use component_scanner::ComponentScanner;

#[derive(Clone)]
pub struct DenseComponentStorage<T>(Vec<Option<T>>);

pub struct DenseComponentScanner<'a, T: 'a> {
    next_index: usize,
    slice: &'a [Option<T>],
}

pub struct DenseComponentScannerMut<'a, T: 'a> {
    next_index: usize,
    iter: slice::IterMut<'a, Option<T>>,
}

impl<T> Default for DenseComponentStorage<T> {
    fn default() -> DenseComponentStorage<T> {
        DenseComponentStorage(Vec::new())
    }
}

impl<'a, T: 'static + Send + Sync + Clone> ComponentStorage<'a> for DenseComponentStorage<T> {
    type Component = T;
    type Scan = DenseComponentScanner<'a, T>;
    type ScanMut = DenseComponentScannerMut<'a, T>;

    fn get(&self, index: usize) -> Option<&T> {
        if index < self.0.len() {
            self.0[index].as_ref()
        } else {
            None
        }
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.0.len() {
            self.0[index].as_mut()
        } else {
            None
        }
    }

    fn insert(&mut self, index: usize, component: T) -> Option<T> {
        while index >= self.0.len() {
            self.0.push(None)
        }
        let slot = &mut self.0[index];
        let old = slot.take();
        *slot = Some(component);
        old
    }

    fn remove(&mut self, index: usize) -> Option<T> {
        if index < self.0.len() {
            self.0[index].take()
        } else {
            None
        }
    }

    fn scan(&'a self) -> Self::Scan {
        DenseComponentScanner {
            next_index: 0,
            slice: &self.0,
        }
    }

    fn scan_mut(&'a mut self) -> Self::ScanMut {
        DenseComponentScannerMut {
            next_index: 0,
            iter: self.0.iter_mut(),
        }
    }
}

impl<T: 'static> DenseComponentStorage<T> {
    pub fn new() -> DenseComponentStorage<T> {
        DenseComponentStorage(Vec::new())
    }
}

impl<'a, T> ComponentScanner for DenseComponentScanner<'a, T> {
    type Item = &'a T;

    fn scan(&mut self, until: Option<usize>) -> Option<(&'a T, usize)> {
        let mut i = cmp::max(self.next_index, until.unwrap_or(0));
        loop {
            if i >= self.slice.len() {
                self.next_index = i + 1;
                return None;
            } else {
                if let Some(ref t) = self.slice[i] {
                    self.next_index = i + 1;
                    return Some((t, i));
                }
            }

            i += 1;
        }
    }
}

impl<'a, T> ComponentScanner for DenseComponentScannerMut<'a, T> {
    type Item = &'a mut T;

    fn scan(&mut self, until: Option<usize>) -> Option<(&'a mut T, usize)> {
        loop {
            let until = until.unwrap_or(0);

            let r;
            let i;
            if until > self.next_index {
                r = self.iter.nth(until - self.next_index);
                i = until;
                self.next_index = until + 1;
            } else {
                r = self.iter.next();
                i = self.next_index;
                self.next_index += 1;
            }

            match r {
                Some(&mut Some(ref mut r)) => {
                    return Some((r, i));
                }
                None => {
                    return None;
                }
                _ => {}
            }
        }
    }
}
