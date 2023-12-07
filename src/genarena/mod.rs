//! Inspired from [generational-arena](https://github.com/fitzgen/generational-arena/), except with a few fundamental
//! changes.
//!
//! * This is not a standalone crate because `smec` needs to do a custom Serde implementation for Arena<T>
//! and see the internals
//! * Generation is stored not in a global (in Arena) variable, but in each entry. This means that
//! the generation will be way less inclined to grow fast, which was a risk with the other code (if thousands
//! or more entites were removed per second, this would be a reaity after a few years. If we want a persistent
//! Arena over a few years, this is a necessity.
//! * When Serializing/Deserializing, empty/free entries are kept (and not filtered out)

#[cfg(feature = "use_serde")]
use serde::{Serialize, Deserialize};

mod iter;
pub use iter::*;
#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct GenArena<T> {
    pub (crate) entries: Vec<Entry<T>>,
    /// Points to the next Free Entry. Free entries are are single-way linked list,
    /// so they may not be in order
    pub (crate) next_free: Option<usize>,
    /// The length of the arena, or the number of `Occupied` variant in entries.
    pub (crate) length: usize,
}

#[derive(Debug)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum Entry<T> {
    Free { next_generation: u64, next_free: Option<usize> },
    Occupied { generation: u64, value: T }
}

impl<T> Entry<T> {
    pub fn map<U, F>(self, f: F) -> Entry<U> where F: FnOnce(T) -> U {
        match self {
            Self::Free { next_generation, next_free } => Entry::Free { next_generation, next_free },
            Self::Occupied { generation, value } => Entry::Occupied { generation, value: f(value) },
        }
    }

    pub fn as_ref(&self) -> Entry<&T> {
        match self {
            Self::Free { next_generation, next_free } => Entry::Free {
                next_generation: *next_generation,
                next_free: *next_free
            },
            Self::Occupied { generation, value } => Entry::Occupied {
                generation: *generation,
                value
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct Index {
    pub index: usize,
    pub generation: u64,
}

impl Index {
    pub fn new(index: usize, generation: u64) -> Self {
        Index { index, generation }
    }
}

impl std::fmt::Display for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#07x}#{:03}", self.index, self.generation)
    }
}

impl std::fmt::Debug for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#08x}#{:04}", self.index, self.generation)
    }
}

impl<T> Default for GenArena<T> {
    fn default() -> GenArena<T> {
        Self::new()
    }
}

pub const DEFAULT_ARENA_CAPACITY: usize = 32;

impl<T> GenArena<T> {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_ARENA_CAPACITY)
    }

    /// Internal usage only.
    ///
    /// Mostly used for EntityList::deserialize
    #[cfg(feature = "use_serde")]
    pub (crate) fn from_raw(entries: Vec<Entry<T>>, length: usize, next_free: Option<usize>) -> Self {
        debug_assert!(length == entries.iter().filter(|e| matches!(e, Entry::Occupied { .. })).count());
        Self {
            entries,
            length,
            next_free
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut arena = GenArena {
            entries: Vec::new(),
            next_free: None,
            length: 0,
        };
        if capacity > 0 {
            arena.reserve_exact(capacity);
        }
        arena
    }

    /// Reserves exactly `added_capacity` new entries, and return the value of `next_free`, without option.
    fn internal_reserve_exact(&mut self, added_capacity: usize) -> usize {
        self.entries.reserve_exact(added_capacity);
        let reserve_start = self.entries.len();
        for i in 0..(added_capacity-1) {
            self.entries.push(Entry::Free { next_generation: 0, next_free: Some(reserve_start + i + 1) });
        }
        self.entries.push(Entry::Free { next_generation: 0, next_free: self.next_free });
        self.next_free = Some(reserve_start);
        reserve_start
    }

    #[inline]
    pub fn reserve_exact(&mut self, added_capacity: usize) {
        self.internal_reserve_exact(added_capacity);
    }

    pub fn clear(&mut self) {
        if let Some((last, head)) = self.entries.split_last_mut() {
            match *last {
                Entry::Free { next_generation, .. } => {
                    *last = Entry::Free { next_generation, next_free: None }
                },
                Entry::Occupied { generation, .. } => {
                    *last = Entry::Free { next_generation: generation + 1, next_free: None }
                }
            }
            for (i, entry) in head.iter_mut().enumerate() {
                match *entry {
                    Entry::Free { next_generation, .. } => {
                        *entry = Entry::Free { next_generation, next_free: Some(i + 1) }
                    },
                    Entry::Occupied { generation, .. } => {
                        *entry = Entry::Free { next_generation: generation + 1, next_free: Some(i + 1) }
                    }
                }
            }
        }
        self.length = 0;
        self.next_free = Some(0);
    }

    /// Force an insert as `index`, panicking if a previous value exists. Internal use only.
    ///
    /// Does nothing if the index is not a valid one.
    fn force_insert_at(&mut self, index: usize, value: T) -> Index {
        if let Entry::Free { next_generation, next_free } = self.entries[index] {
            self.entries[index] = Entry::Occupied { generation: next_generation, value };
            self.next_free = next_free;
            self.length += 1;
            Index { generation: next_generation, index }
        } else {
            panic!("index {index} in genarena is already occupied for force_insert_at");
        }
    }

    /// Push `T` into the arena.
    pub fn push(&mut self, value: T) -> Index {
        match self.next_free {
            Some(next_free) => {
                self.force_insert_at(next_free, value)
            },
            None => {
                const MIN_RESERVE: usize = 8;
                // reserve to have double the amount we have, but there is a special case:
                // if the amount we have is zero, double zero is zero. For that case, we have a minimum reserve
                // constant just above.
                let next_free = self.internal_reserve_exact(std::cmp::max(self.entries.len(), MIN_RESERVE));
                self.force_insert_at(next_free, value)
            }
        }
    }

    pub fn remove(&mut self, index: Index) -> Option<T> {
        if let Some(entry) = self.entries.get_mut(index.index) {
            let Entry::Occupied { generation, .. } = entry else {
                return None;
            };
            if *generation != index.generation {
                return None;
            }
            let new_entry = Entry::Free { next_generation: *generation + 1, next_free: self.next_free };
            let removed_entry = std::mem::replace(entry, new_entry);
            self.next_free = Some(index.index);
            self.length -= 1;
            if let Entry::Occupied { value, .. } = removed_entry {
                Some(value)
            } else {
                unreachable!("removed entity in remove is not Occupied variant")
            }
        } else {
            None
        }
    }

    #[inline]
    pub fn contains(&self, index: Index) -> bool {
        self.get(index).is_some()
    }

    pub fn get(&self, index: Index) -> Option<&T> {
        if let Some(Entry::Occupied { generation, value }) = self.entries.get(index.index) {
            if *generation != index.generation {
                return None;
            }
            Some(value)
        } else {
            None
        }
    }

    /// Get a value and its generation from an `usize` index (without generation)
    pub fn get_raw(&self, index: usize) -> Option<(&T, u64)> {
        if let Some(Entry::Occupied { generation, value }) = self.entries.get(index) {
            Some((value, *generation))
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: Index) -> Option<&mut T> {
        if let Some(Entry::Occupied { generation, value }) = self.entries.get_mut(index.index) {
            if *generation != index.generation {
                return None;
            }
            Some(value)
        } else {
            None
        }
    }

    /// Get a mutable value and its generation from an `usize` index (without generation)
    pub fn get_raw_mut(&mut self, index: usize) -> Option<(&mut T, u64)> {
        if let Some(Entry::Occupied { generation, value }) = self.entries.get_mut(index) {
            Some((value, *generation))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            entries: &self.entries,
            tot_length: self.length,
            seen: 0,
            curr: 0,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            entries: &mut self.entries,
            tot_length: self.length,
            seen: 0,
            curr: 0,
        }
    }

    pub fn values(&self) -> impl Iterator<Item=&T> {
        self.iter().map(|(_i, v)| v)
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item=&mut T> {
        self.iter_mut().map(|(_i, v)| v)
    }

    pub fn capacity(&self) -> usize {
        self.entries.len()
    }
}

impl<T:Clone> Clone for GenArena<T> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            next_free: self.next_free,
            length: self.length
        }
    }

    fn clone_from(&mut self, other: &Self) {
        self.entries.clone_from(&other.entries);
        self.next_free = other.next_free;
        self.length = other.length;
    }
}

impl<T: Clone> Clone for Entry<T> {
    fn clone(&self) -> Self {
        match self {
            Entry::Free { next_free, next_generation } => Entry::Free {
                next_free: *next_free,
                next_generation: *next_generation
            },
            Entry::Occupied { generation, value } => Entry::Occupied {
                generation: *generation,
                value: value.clone(),
            }
        }
    }

    fn clone_from(&mut self, other: &Self) {
        match (self, other) {
            (
                Entry::Occupied { generation: dest_gen, value: dest_value },
                Entry::Occupied { generation, value }
            ) => {
                *dest_gen = *generation;
                dest_value.clone_from(value);
                return;
            },
            (s, o) => { *s = o.clone() }
        }
    }
}

impl<T> std::ops::Index<Index> for GenArena<T> {
    type Output = T;

    fn index(&self, index: Index) -> &Self::Output {
        self.get(index).expect("GenArena.index(Index): no element found at index")
    }
}

impl<T> std::ops::IndexMut<Index> for GenArena<T> {
    fn index_mut(&mut self, index: Index) -> &mut Self::Output {
        self.get_mut(index).expect("GenArena.index_mut(Index): no element found at index")
    }
}
