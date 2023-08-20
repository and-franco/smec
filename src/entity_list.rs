use std::any::TypeId;
use std::convert::TryInto;
use std::cell::UnsafeCell;
use std::rc::Rc;

use hashbrown::HashMap;
use hibitset::{BitSet};

use crate::genarena::{GenArena, Index};

use crate::{EntityBase, EntityRefBase, EntityOwnedBase, Component, ComponentsStorage};

pub type EntityId = Index;

/// The struct holding a list/array of entities.
///
/// It is backed by a `generational_arena`, and a `hibitset`.
///
/// It has the following properties:
///
/// * Creations and removals are mostly `O(1)`
/// * Iteration is linear time (unless you specify the components you're looking for,
/// where it is at worse the same, at best hundreds of time faster, thanks to hibitset).
/// * IDs cannot be reused, but their memory space is reusable.
pub struct EntityList<E: EntityRefBase> {
    pub (crate) bitsets: HashMap<TypeId, BitSet>,
    pub (crate) entities: GenArena<E>,
    pub components_storage: Rc<UnsafeCell<E::CS>>,
}

impl<E: EntityRefBase> EntityList<E> {
    pub fn new() -> EntityList<E> {
        let components_storage = <<E as EntityRefBase>::CS as ComponentsStorage>::new();
        let mut l = EntityList {
            bitsets: HashMap::new(),
            entities: GenArena::new(),
            components_storage: Rc::new(UnsafeCell::new(components_storage))
        };
        l.init_bitsets(None);
        l
    }

    /// Insert an entity.
    ///
    /// Returns the ID of the entity you've just inserted.
    pub fn insert(&mut self, entity: E::Owned) -> EntityId {
        let mut type_ids: Vec<TypeId> = Vec::with_capacity(8);
        entity.for_each_active_component(|type_id: TypeId| {
            type_ids.push(type_id);
        });
        let entity_id = self.entities.push(EntityRefBase::from_owned(entity, &self.components_storage));
        for type_id in type_ids {
            if let Some(bitset) = self.bitsets.get_mut(&type_id) {
                bitset.add(entity_id.index as u32);
            }
        }
        entity_id
    }

    /// Remove an entity
    ///
    /// If the entity wasn't already removed, it is returned as an `Option`.
    pub fn remove(&mut self, id: EntityId) -> Option<E::Owned> {
        if let Some(e) = self.entities.remove(id) {
            e.for_each_active_component(|type_id: TypeId| {
                if let Some(bitset) = self.bitsets.get_mut(&type_id) {
                    bitset.remove(id.index as u32);
                }
            });
            unsafe {
                let cs = &mut *self.components_storage.get();
                Some(e.to_owned(cs))
            }
        } else {
            None
        }
    }

    /// Refresh bitset for an entity
    ///
    /// You need to call this after a `.add::<C>()` or `.remove::<C>()`
    pub fn refresh(&mut self, id: EntityId) {
        println!("refresh {:?}", id);
        if let Some(e) = self.entities.get_mut(id) {
            let bitsets = &mut self.bitsets;
            e.for_each_component(|type_id: TypeId, is_active: bool| {
                dbg!(type_id, is_active);
                if let Some(bitset) = bitsets.get_mut(&type_id) {
                    if is_active {
                        bitset.add(id.index as u32);
                    } else {
                        bitset.remove(id.index as u32);
                    }
                }
            });
        }
    }

    #[inline]
    /// Retrives an entity immutably.
    pub fn get(&self, id: EntityId) -> Option<&E> {
        self.entities.get(id)
    }

    #[inline]
    /// Retrieves an entity mutably.
    ///
    /// **WARNING**: You must not add or remove a component to this entity via the mutable
    /// reference, otherwise the bitset cache will be invalid, resulting in this entity
    /// possibly not being iterated over!
    ///
    /// To add or remove a component for an entity, use `add_component_for_entity` and
    /// `remove_component_for_entity`.
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut E> {
        self.entities.get_mut(id)
    }

    #[inline]
    /// Returns true if the id exists.
    pub fn contains(&self, id: EntityId) -> bool {
        self.entities.contains(id)
    }

    #[inline]
    /// Returns the number of entities in the list.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Initialize bitsets for all components of entity E
    ///
    /// Default capacity is 4096, and is applied for all bitsets.
    pub (crate) fn init_bitsets(&mut self, capacity: Option<u32>) {
        E::for_all_components(|type_id: TypeId| {
            self.bitsets.insert(type_id, BitSet::with_capacity(capacity.unwrap_or(4096)));
        });
    }

    #[allow(dead_code)] // we might find a use for it in the future, it used to be used in EntityList::from_arena
    /// In case the bitsets are out of date, this function can re-generate them.
    fn regenerate_all_component_bitsets(&mut self) {
        let capacity = self.entities.len();

        E::for_all_components(|type_id: TypeId| {
            self.bitsets.insert(type_id, BitSet::with_capacity(capacity as u32));
        });
        let mut bitsets: Vec<(TypeId, &mut BitSet)> = self.bitsets.iter_mut().map(|(k, v)| (*k, v)).collect::<Vec<_>>();
        bitsets.sort_unstable_by(|(k1, _), (k2, _)| k1.cmp(k2));
        for (id, el) in &self.entities {
            el.for_each_active_component(|seek_type_id: TypeId| {
                if let Ok(i) = bitsets.binary_search_by(|(tid, _)| tid.cmp(&seek_type_id)) {
                    bitsets[i].1.add(id.index as u32);
                } else {
                    unreachable!()
                }
            })
        }
    }

    // Add a bitset for a specific component for all entities.
    //
    // Typically done at the very start of the ECS
    #[allow(dead_code)]
    pub (crate) fn add_bitset_for_component<C: Component<E>>(&mut self) {
        let bitset_capacity: u32 = self.entities.capacity().try_into().expect("too many entities");
        let mut bitset = BitSet::with_capacity(bitset_capacity);
        for (entity_id, entity) in &self.entities {
            if entity.has::<C>() {
                bitset.add(entity_id.index as u32);
            }
        }
        self.bitsets.insert(
            TypeId::of::<C>(),
            bitset
        );
    }

    // Remove a bitset for a specific component for all entities.
    //
    // Returns true if the bitset was actually there and was removed
    #[allow(dead_code)]
    pub (crate) fn remove_bitset_for_component<C: Component<E>>(&mut self) -> bool {
        let bitset_capacity: u32 = self.entities.capacity().try_into().expect("too many entities");
        let mut bitset = BitSet::with_capacity(bitset_capacity);
        for (entity_id, entity) in &self.entities {
            if entity.has::<C>() {
                bitset.remove(entity_id.index as u32);
            }
        }
        self.bitsets.remove(
            &TypeId::of::<C>()
        ).is_some()
    }

    /// Add a component for the given entity.
    ///
    /// If the entity does not exist anymore, `Some(component)` is returned.
    pub fn add_component_for_entity<C: Component<E>>(&mut self, entity_id: EntityId, component: C) -> Option<C> {
        let maybe_component = match self.entities.get_mut(entity_id) {
            Some(e) => {
                component.set(e);
                None
            },
            None => {
                Some(component)
            }
        };
        // maybe_component is Some if it hasn't been applied, None if it has been applied.
        if maybe_component.is_none() {
            // if it has been added, see if we have a bitset for this component
            if let Some(bitset) = self.bitsets.get_mut(&TypeId::of::<C>()) {
                // we have a bitset, so add the info that this entity has the given component
                bitset.add(entity_id.index as u32);
            };
        };

        maybe_component
    }

    /// Remove a component for the given entity.
    ///
    /// If the entity exists and it has the component, `Some(component)` is returned.
    pub fn remove_component_for_entity<C: Component<E>>(&mut self, entity_id: EntityId) -> Option<Box<C>> {
        let maybe_component = self.entities
            .get_mut(entity_id)
            .and_then(C::remove);

        // maybe_component is Some if it was a component, None if it wasn't.
        if maybe_component.is_some() {
            // if it has been removed, see if we have a bitset for this component
            if let Some(bitset) = self.bitsets.get_mut(&TypeId::of::<C>()) {
                // we have a bitset, so remove the info that this entity has the given component
                bitset.remove(entity_id.index as u32);
            };
        };

        maybe_component
    }
}

impl<E: EntityRefBase> std::fmt::Debug for EntityList<E> where E: std::fmt::Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.entities.fmt(f)
    }
}

impl<E: EntityRefBase> Clone for EntityList<E> {
    fn clone(&self) -> EntityList<E> {
        let cloned_cs = unsafe { (&*self.components_storage.get()).clone() };
        let cs = Rc::new(UnsafeCell::new(cloned_cs));
        let mut gen_arena = self.entities.clone();
        for entity in gen_arena.values_mut() {
            entity.set_cs(Rc::downgrade(&cs))
        }
        EntityList {
            bitsets: self.bitsets.clone(),
            entities: gen_arena,
            components_storage: cs,
        }
    }

    fn clone_from(&mut self, other: &Self) {
        self.bitsets.clone_from(&other.bitsets);
        unsafe {
            let self_cs: &mut E::CS = &mut *self.components_storage.get();
            let other_cs: &E::CS = &*other.components_storage.get();
            self_cs.clone_from(&other_cs);
        }
        self.entities.clone_from(&other.entities);
        for entity in self.entities.values_mut() {
            entity.set_cs(Rc::downgrade(&self.components_storage))
        }
    }
}