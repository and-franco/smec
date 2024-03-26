use crate::{
    Component, RefComponent, EntityBase, EntityRefBase, EntityOwnedBase, EntityList, EntityId,
    genarena::{GenArena}
};
use slab::Slab;
use hibitset::{BitIter, BitSet, BitSetLike, BitSetAll, BitSetAnd};
use tuple_utils::Split;

use std::any::TypeId;

use hashbrown::HashMap;

impl<E: EntityRefBase> EntityList<E> {
    /// Iterate over all entities
    pub fn iter_all<'a>(&'a self) -> impl Iterator<Item=(EntityId, &'a E)> + Clone {
        self.entities.iter()
    }

    /// Iterate over all entities mutably
    pub fn iter_all_mut<'a>(&'a mut self) -> impl Iterator<Item=(EntityId, &'a mut E)> {
        self.entities.iter_mut()
    }

    /// Iterate over all entities which have the component `C`, immutably.
    ///
    /// There is no mutable version of this, use iter::<(C,)>() if you need one
    pub fn iter_single<'a, C: RefComponent<E>>(&'a self) -> SingleComponentIter<'a, E, C> {
        SingleComponentIter::new(self)
    }

    /// Iterate over all entities which have the components (C1, C2, C3, ...)
    /// 
    /// Even if you want only one component, it must be a tuple.
    /// 
    /// # Example
    /// 
    /// `for (id, entity) in entities.iter::<(Speed,)>() { }`
    pub fn iter<'a, C: MultiComponent<'a, E>>(&'a self) -> MultiComponentIter<'a, E, C::BitSet> {
        C::iter(&self.bitsets, &self.entities)
    }

    /// Iterate over all entities which have the components (C1, C2, C3, ...), mutably
    /// 
    /// # Example
    /// 
    /// `for (id, entity) in entities.iter_mut::<(Speed, Gravity)>() { }`
    pub fn iter_mut<'a, C: MultiComponent<'a, E>>(&'a mut self) -> MultiComponentIterMut<'a, E, C::BitSet> {
        C::iter_mut(&self.bitsets, &mut self.entities)
    }
}

pub struct SingleComponentIter<'a, E: EntityRefBase, C: Component<E>> {
    pub (crate) iter: BitIter<&'a BitSet>,
    pub (crate) values: &'a GenArena<E>,
    pub (crate) slab_ref: &'a Slab<C>,
}

impl<'a, E: EntityRefBase, C: Component<E>> Clone for SingleComponentIter<'a, E, C> {
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            values: self.values,
            slab_ref: self.slab_ref,
        }
    }
}

impl<'a, E: EntityRefBase, C: RefComponent<E>> SingleComponentIter<'a, E, C> {
    pub fn new(list: &'a EntityList<E>) -> SingleComponentIter<'a, E, C> {
        let bitset = list.bitsets.get(&TypeId::of::<C>()).expect("FATAL: bitset is non-existant for composant");
        let cs_ref: &E::CS = unsafe { &*list.components_storage.get() };
        let slab_ref: &Slab<C> = C::get_single_cs(cs_ref);
        SingleComponentIter {
            iter: bitset.iter(),
            values: &list.entities,
            slab_ref,
        }
    }
}

impl<'a, E: EntityBase, B: BitSetLike> Iterator for MultiComponentIter<'a, E, B> {
    type Item = (EntityId, &'a E);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|index| {
            self.values.get_raw(index as usize)
                .map(|(v, g)| (EntityId::new(index as usize, g), v))
                .expect(FATAL_ERR_BITSET)
        })
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

const FATAL_ERR_BITSET: &str = r##"
    !!!!FATAL: bitset is out of date, bitset returned true for an entity, but no entity exists at this location!!!! \
    Check that your code adds components and entities via the legal methods!"
"##;
const FATAL_ERR_CS: &str = r##"!!!!FATAL: Component Storage does not have content that is referenced by entity!!!!"##;

impl<'a, E: EntityRefBase, C: RefComponent<E>> Iterator for SingleComponentIter<'a, E, C> {
    type Item = (EntityId, &'a E, &'a C);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|index| {
            self.values.get_raw(index as usize)
                .map(|(v, g)| (
                    EntityId::new(index as usize, g),
                    v,
                    self.slab_ref.get(C::get_cs_id(v).expect(FATAL_ERR_BITSET))
                    .expect(FATAL_ERR_CS)
                ))
                .expect(FATAL_ERR_BITSET)
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
                .expect(FATAL_ERR_BITSET);
        
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