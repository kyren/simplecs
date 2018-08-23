use std::iter::FromIterator;
use std::{iter, slice, vec};

/// A unique identifier with an associated usize index.  Indexes are valued proportional to the
/// number of indexes allocated, are reused after being freed, and do not grow without bound.  When
/// an index is re-used, an associated "generation" is incremented, so that within the life of a
/// single allocator, no two GenerationalIndex values will ever be equal.  Since the indexes do not
/// grow without bound, GenerationalIndex values are particularly suited to being stored by their
/// index in extremely fast contiguous arrays.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct GenerationalIndex {
    index: usize,
    generation: u64,
}

/// Allocates GenerationalIndexes without duplication.
#[derive(Clone, Default)]
pub struct GenerationalIndexAllocator {
    entries: Vec<AllocatorEntry>,
    free: Vec<usize>,
}

/// An associative array of GenerationalIndex keys to values.  Takes advantage of how
/// GenerationalIndex indexes work to very efficiently map values in a contiguous array.  Generally
/// only efficient when storing lots of entries for a long time, as it has storage requirements
/// proportional to the largest index encountered.
#[derive(Clone, Default)]
pub struct GenerationalIndexArray<T>(Vec<Option<ArrayEntry<T>>>);

pub struct GenerationalIndexArrayIter<'a, T: 'a>(
    iter::Enumerate<slice::Iter<'a, Option<ArrayEntry<T>>>>,
);
pub struct GenerationalIndexArrayIterMut<'a, T: 'a>(
    iter::Enumerate<slice::IterMut<'a, Option<ArrayEntry<T>>>>,
);
pub struct GenerationalIndexArrayIntoIter<T>(iter::Enumerate<vec::IntoIter<Option<ArrayEntry<T>>>>);

#[derive(Clone)]
struct AllocatorEntry {
    is_live: bool,
    generation: u64,
}

#[derive(Clone)]
struct ArrayEntry<T> {
    value: T,
    generation: u64,
}

impl GenerationalIndex {
    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

impl GenerationalIndexAllocator {
    pub fn new() -> GenerationalIndexAllocator {
        Default::default()
    }

    pub fn allocate(&mut self) -> GenerationalIndex {
        if let Some(index) = self.free.pop() {
            let id_entry = &mut self.entries[index];
            assert!(!id_entry.is_live);
            id_entry.is_live = true;
            GenerationalIndex {
                index: index,
                generation: id_entry.generation,
            }
        } else {
            self.entries.push(AllocatorEntry {
                is_live: true,
                generation: 0,
            });
            GenerationalIndex {
                index: self.entries.len() - 1,
                generation: 0,
            }
        }
    }

    pub fn deallocate(&mut self, gen_index: GenerationalIndex) -> bool {
        if gen_index.index >= self.entries.len() {
            return false;
        }

        let id_entry = &mut self.entries[gen_index.index];
        if !id_entry.is_live {
            return false;
        }

        id_entry.is_live = false;
        id_entry.generation = id_entry
            .generation
            .checked_add(1)
            .expect("GenerationalIndex generation overflow");
        self.free.push(gen_index.index);
        true
    }

    #[inline]
    pub fn is_live(&self, gen_index: GenerationalIndex) -> bool {
        if gen_index.index < self.entries.len() {
            let id_entry = &self.entries[gen_index.index];
            id_entry.is_live && id_entry.generation == gen_index.generation
        } else {
            false
        }
    }

    /// Returns the maximum index ever allocated so far.
    #[inline]
    pub fn max_allocated_index(&self) -> usize {
        self.entries.len()
    }

    /// If there is a live GenerationalIndex for the given index, returns it.  All entries past
    /// max_allocated_index will return None.
    #[inline]
    pub fn live_at_index(&self, index: usize) -> Option<GenerationalIndex> {
        self.entries.get(index).and_then(|entry| {
            if entry.is_live {
                Some(GenerationalIndex {
                    index,
                    generation: self.entries[index].generation,
                })
            } else {
                None
            }
        })
    }
}

impl<T> GenerationalIndexArray<T> {
    pub fn new() -> GenerationalIndexArray<T> {
        GenerationalIndexArray(Vec::new())
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Overwrites any entry with the matching index, returns both the GenerationalIndex and T that
    /// were replaced, which may be a GenerationalIndex from a past generation.
    pub fn insert(
        &mut self,
        gen_index: GenerationalIndex,
        value: T,
    ) -> Option<(GenerationalIndex, T)> {
        if gen_index.index >= self.0.len() {
            for _ in self.0.len()..gen_index.index + 1 {
                self.0.push(None);
            }
        }

        let entry = &mut self.0[gen_index.index];

        let old = entry.take().map(|e| {
            (
                GenerationalIndex {
                    index: gen_index.index,
                    generation: e.generation,
                },
                e.value,
            )
        });
        *entry = Some(ArrayEntry {
            value,
            generation: gen_index.generation,
        });
        old
    }

    pub fn remove(&mut self, gen_index: GenerationalIndex) -> Option<T> {
        if gen_index.index < self.0.len() {
            let entry = &mut self.0[gen_index.index];

            if let Some(e) = entry.take() {
                if e.generation == e.generation {
                    return Some(e.value);
                } else {
                    *entry = Some(e);
                }
            }
        }
        None
    }

    pub fn contains_key(&self, gen_index: GenerationalIndex) -> bool {
        self.get(gen_index).is_some()
    }

    pub fn get(&self, gen_index: GenerationalIndex) -> Option<&T> {
        if gen_index.index < self.0.len() {
            self.0[gen_index.index].as_ref().and_then(|e| {
                if e.generation == gen_index.generation {
                    Some(&e.value)
                } else {
                    None
                }
            })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, gen_index: GenerationalIndex) -> Option<&mut T> {
        if gen_index.index < self.0.len() {
            self.0[gen_index.index].as_mut().and_then(|e| {
                if e.generation == gen_index.generation {
                    Some(&mut e.value)
                } else {
                    None
                }
            })
        } else {
            None
        }
    }

    pub fn retain<F: FnMut(GenerationalIndex, &mut T) -> bool>(&mut self, mut f: F) {
        for i in 0..self.0.len() {
            let entry = &mut self.0[i];

            let keep = if let Some(entry) = entry.as_mut() {
                f(
                    GenerationalIndex {
                        index: i,
                        generation: entry.generation,
                    },
                    &mut entry.value,
                )
            } else {
                false
            };

            if !keep {
                *entry = None;
            }
        }
    }

    pub fn filter_map<F: FnMut(GenerationalIndex, T) -> Option<T>>(&mut self, mut f: F) {
        for i in 0..self.0.len() {
            let entry = &mut self.0[i];

            if let Some(e) = entry.take() {
                let gen_index = GenerationalIndex {
                    index: i,
                    generation: e.generation,
                };

                if let Some(value) = f(gen_index, e.value) {
                    *entry = Some(ArrayEntry {
                        value,
                        generation: gen_index.generation,
                    })
                }
            }
        }
    }

    pub fn iter<'a>(&'a self) -> GenerationalIndexArrayIter<'a, T> {
        GenerationalIndexArrayIter(self.0.iter().enumerate())
    }

    pub fn iter_mut<'a>(&'a mut self) -> GenerationalIndexArrayIterMut<'a, T> {
        GenerationalIndexArrayIterMut(self.0.iter_mut().enumerate())
    }
}

impl<'a, T: 'a> Iterator for GenerationalIndexArrayIter<'a, T> {
    type Item = (GenerationalIndex, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((index, entry)) = self.0.next() {
            if let &Some(ref entry) = entry {
                return Some((
                    GenerationalIndex {
                        index,
                        generation: entry.generation,
                    },
                    &entry.value,
                ));
            }
        }
        None
    }
}

impl<'a, T: 'a> Iterator for GenerationalIndexArrayIterMut<'a, T> {
    type Item = (GenerationalIndex, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((index, entry)) = self.0.next() {
            if let &mut Some(ref mut entry) = entry {
                return Some((
                    GenerationalIndex {
                        index,
                        generation: entry.generation,
                    },
                    &mut entry.value,
                ));
            }
        }
        None
    }
}

impl<T> Iterator for GenerationalIndexArrayIntoIter<T> {
    type Item = (GenerationalIndex, T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((index, entry)) = self.0.next() {
            if let Some(entry) = entry {
                return Some((
                    GenerationalIndex {
                        index,
                        generation: entry.generation,
                    },
                    entry.value,
                ));
            }
        }
        None
    }
}

impl<'a, T: 'a> IntoIterator for &'a GenerationalIndexArray<T> {
    type Item = (GenerationalIndex, &'a T);
    type IntoIter = GenerationalIndexArrayIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: 'a> IntoIterator for &'a mut GenerationalIndexArray<T> {
    type Item = (GenerationalIndex, &'a mut T);
    type IntoIter = GenerationalIndexArrayIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T> IntoIterator for GenerationalIndexArray<T> {
    type Item = (GenerationalIndex, T);
    type IntoIter = GenerationalIndexArrayIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        GenerationalIndexArrayIntoIter(self.0.into_iter().enumerate())
    }
}

impl<T> FromIterator<(GenerationalIndex, T)> for GenerationalIndexArray<T> {
    fn from_iter<I: IntoIterator<Item = (GenerationalIndex, T)>>(
        iter: I,
    ) -> GenerationalIndexArray<T> {
        let mut map = GenerationalIndexArray::new();
        for (entity, value) in iter {
            map.insert(entity, value);
        }
        map
    }
}
