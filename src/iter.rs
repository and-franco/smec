use crate::{
    Component, EntityBase, EntityRefBase, EntityOwnedBase, EntityList, EntityId,
    genarena::{GenArena}
};
use hibitset::{BitIter, BitSet, BitSetLike, BitSetAll, BitSetAnd};
use tuple_utils::Split;

use std::any::TypeId;

use hashbrown::HashMap;

impl<E: EntityRefBase> EntityList<E> {
    pub fn iter_all<'a>(&'a self) -> impl Iterator<Item=(EntityId, &'a E)> {
        self.entities.iter()
    }

    pub fn iter_all_mut<'a>(&'a mut self) -> impl Iterator<Item=(EntityId, &'a mut E)> {
        self.entities.iter_mut()
    }

    pub fn iter<'a, C: MultiComponent<'a, E>>(&'a self) -> MultiComponentIter<'a, E, C::BitSet> {
        C::iter(&self.bitsets, &self.entities)
    }

    pub fn iter_mut<'a, C: MultiComponent<'a, E>>(&'a mut self) -> MultiComponentIterMut<'a, E, C::BitSet> {
        C::iter_mut(&self.bitsets, &mut self.entities)
    }
}

pub struct MultiComponentIter<'a, E: EntityBase, B: BitSetLike> {
    pub (crate) iter: BitIter<B>,
    pub (crate) values: &'a GenArena<E>,
}

impl<'a, E: EntityBase, B: BitSetLike> MultiComponentIter<'a, E, B> {
    pub fn new(iter: BitIter<B>, values: &'a GenArena<E>) -> Self {
        MultiComponentIter {
            iter,
            values,
        }
    }
}

pub struct MultiComponentIterMut<'a, E: EntityBase, B: BitSetLike> {
    pub (crate) iter: BitIter<B>,
    pub (crate) values: &'a mut GenArena<E>,
    #[cfg(debug_assertions)]
    pub (crate) n: Option<usize>,
}

impl<'a, E: EntityBase, B: BitSetLike> MultiComponentIterMut<'a, E, B> {
    pub fn new(iter: BitIter<B>, values: &'a mut GenArena<E>) -> Self {
        MultiComponentIterMut {
            iter,
            values,
            #[cfg(debug_assertions)]
            n: None,
        }
    }
}

const FATAL_ERR: &str = r##"
    !!!!FATAL: bitset is out of date, bitset returned true for an entity, but no entity exists at this location!!!! \
    Check that your code adds components and entities via the legal methods!"
"##;

impl<'a, E: EntityBase, B: BitSetLike> Iterator for MultiComponentIter<'a, E, B> {
    type Item = (EntityId, &'a E);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|index| {
            self.values.get_raw(index as usize)
                .map(|(v, g)| (EntityId::new(index as usize, g), v))
                .expect(FATAL_ERR)
        })
    }
}

impl<'a, E: EntityBase, B: BitSetLike> Iterator for MultiComponentIterMut<'a, E, B> {
    type Item = (EntityId, &'a mut E);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|index| {
            let index = index as usize;
            let (id, v) = self.values.get_raw_mut(index)
                .map(|(v, g)| (EntityId::new(index, g), v))
                .expect(FATAL_ERR);
        
            #[cfg(debug_assertions)] {
                // check that n is strictly monotonic increasing,
                // meaning that the same value will never be indexed twice,
                // THEREFORE we can safely allow the unsafe code below, that unlinks
                // the lifetime of the source with the lifetime of the Iterator::Item
                // we still cannot make the items of the iterator outlive the source,
                // nor can we mutate the source object, but at least we can call .next() safely.
                if let Some(old_n) = self.n {
                    debug_assert!(old_n < index);
                }
                self.n = Some(index);
            }
            
            #[allow(unsafe_code)]
            (id, unsafe { &mut *(v as *mut _) }) 
        })
    }
}

/// Trait used internally, implemented for every tuple of component.
///
/// Do not implement externally.
pub trait MultiComponent<'a, E: EntityBase> {
    type BitSet: BitSetLike;

    fn bitset(bitsets: &'a HashMap<TypeId, BitSet>) -> Self::BitSet;

    fn iter(bitsets: &'a HashMap<TypeId, BitSet>, arena: &'a GenArena<E>) -> MultiComponentIter<'a, E, Self::BitSet> {
        MultiComponentIter::new(Self::bitset(bitsets).iter(), arena)
    }

    fn iter_mut(bitsets: &'a HashMap<TypeId, BitSet>, arena: &'a mut GenArena<E>) -> MultiComponentIterMut<'a, E, Self::BitSet> {
        MultiComponentIterMut::new(Self::bitset(bitsets).iter(), arena)
    }
}

impl<'a, E: EntityBase> MultiComponent<'a, E> for () {
    type BitSet = BitSetAll;

    fn bitset(_bitsets: &'a HashMap<TypeId, BitSet>) -> Self::BitSet {
        BitSetAll
    }
}

impl<'a, E: EntityBase, C: Component<E>> MultiComponent<'a, E> for (C,) {
    type BitSet = &'a BitSet;

    fn bitset(bitsets: &'a HashMap<TypeId, BitSet>) -> Self::BitSet {
        bitsets.get(&TypeId::of::<C>()).expect("FATAL: bitset is non-existant for composant")
    }
}

macro_rules! multi_component_impl {
    // use variables to indicate the arity of the tuple
    ($($ty:ident),*) => {
        impl<'a, E: EntityBase, $($ty: Component<E>),*> MultiComponent<'a, E> for ($($ty),*)
        {
            type BitSet = BitSetAnd<
                <<Self as Split>::Left as MultiComponent<'a, E>>::BitSet,
                <<Self as Split>::Right as MultiComponent<'a, E>>::BitSet
            >;

            fn bitset(bitsets: &'a HashMap<TypeId, BitSet>) -> Self::BitSet {
                let (l, r) = (
                    <<Self as Split>::Left as MultiComponent<'a, E>>::bitset(bitsets),
                    <<Self as Split>::Right as MultiComponent<'a, E>>::bitset(bitsets)
                );
                BitSetAnd(l, r)
            }
        }
    }
}

multi_component_impl!(C1, C2);
multi_component_impl!(C1, C2, C3);
multi_component_impl!(C1, C2, C3, C4);
multi_component_impl!(C1, C2, C3, C4, C5);
multi_component_impl!(C1, C2, C3, C4, C5, C6);
multi_component_impl!(C1, C2, C3, C4, C5, C6, C7);
multi_component_impl!(C1, C2, C3, C4, C5, C6, C7, C8);
multi_component_impl!(C1, C2, C3, C4, C5, C6, C7, C8, C9);
multi_component_impl!(C1, C2, C3, C4, C5, C6, C7, C8, C9, C10);
multi_component_impl!(C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11);
multi_component_impl!(C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12);
multi_component_impl!(C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13);
multi_component_impl!(C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14);
multi_component_impl!(C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15);
multi_component_impl!(C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15, C16);