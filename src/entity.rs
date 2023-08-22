use crate::{ComponentsStorage};
use std::any::TypeId;
use slab::Slab;

pub trait Component<E: Sized>: 'static + Clone {
    fn set(self, entity: &mut E);

    fn get(entity: &E) -> Option<&Self>;

    fn get_mut(entity: &mut E) -> Option<&mut Self>;

    /// Delete a component from an entity
    fn remove(entity: &mut E) -> Option<Box<Self>>;
    
    // read a component with the given predicate. You may return a custom result of your choice.
    fn peek<O, F: FnOnce(&Self) -> O>(entity: &E, f: F) -> Option<O>;

    // update component with the given predicate. You may return a custom result of your choice.
    fn update<O, F: FnOnce(&mut Self) -> O>(entity: &mut E, f: F) -> Option<O>;
}

pub trait RefComponent<E: Sized + EntityRefBase>: Component<E> {
    fn get_single_cs(cs: &E::CS) -> &Slab<Self>;

    fn get_cs_id(entity: &E) -> Option<usize>;
}

pub enum ChangeComponent<C> {
    /// Do not change the given component
    NoChange,
    /// Replace the given component by a new one. Works even if there was no component to begin with.
    Replace(C),
    /// Mutate the currently available component. Only works if there is a component to begin with.
    Mutate(Box<dyn FnOnce(&mut C)>),
    /// Remove the component without adding a new one.
    Remove,
}

pub trait EntityOwnedBase: EntityBase {
    /// CreationParams are always the properties of an entity.
    type CreationParams;

    /// Creates an entity with the given properties.
    ///
    /// Entity::new takes as arguments the properties as tuple in order.
    ///
    /// For instance:
    /// * for no properties, the empty tuple is expected,
    /// * for a single property A, the param is (A,)
    /// * for a two properties A and B, the param is (A, B)
    /// * and so on
    fn new(params: Self::CreationParams) -> Self;
}

pub trait EntityRefBase: EntityBase + Clone {
    type CS: ComponentsStorage;
    // naked is the Ref struct but without the component storage part, used for serializing
    type Naked: Clone;
    type Owned: EntityOwnedBase;

    fn from_owned(owned: Self::Owned, cs: &std::rc::Rc<std::cell::UnsafeCell<Self::CS>>) -> Self;

    fn to_owned(self, cs: &mut Self::CS) -> Self::Owned;

    fn from_naked(naked: Self::Naked, cs: &std::rc::Rc<std::cell::UnsafeCell<Self::CS>>) -> Self;

    fn as_naked(&self) -> Self::Naked;

    fn set_cs(&mut self, cs: std::rc::Weak<std::cell::UnsafeCell<Self::CS>>);
}

pub trait EntityBase: Sized + 'static {
    // For a specific entity, go through every component this entity has.
    fn for_each_active_component(&self, f: impl FnMut(TypeId));

    // For a specific entity, go through every component this entity may have. A boolean
    // is attached to know whether the component is actually there or not.
    fn for_each_component(&self, f: impl FnMut(TypeId, bool));

    // Go through all possible components this kind of entity might have.
    fn for_all_components(f: impl FnMut(TypeId));

    #[inline]
    /// Returns the ntity with the specified component. The old component is discarded.
    fn with<C: Component<Self>>(mut self, component: C) -> Self {
        component.set(&mut self);
        self
    }

    #[inline]
    /// Mutates the component for the given entity.
    ///
    /// Mutations only apply to inner changes, not removal or creation of components. The predicate
    /// is only called if the component exists for the given entity to begin with.
    fn with_mutation<C: Component<Self>, F: FnOnce(&mut C)>(mut self, f: F) -> Self {
        self.mutate(f);
        self
    }

    /// Returns the mutable component if there is one, otherwise fills default and then returns it
    fn get_mut_or_default<C: Component<Self> + Default>(&mut self) -> &mut C {
        if C::get(self).is_none() {
            C::default().set(self);
        };
        // we defined the unit just above if it didn't exist
        self.get_mut::<C>().unwrap()
    }

    /// Executes a fn on the component if there is one, otherwise fills default and then executes the fn
    fn mutate_or_default<C: Component<Self> + Default, O, F: FnOnce(&mut C) -> O>(&mut self, f: F) -> O {
        let c = self.get_mut_or_default();
        f(c)
    }

    #[inline]
    /// Mutates the component for the given entity.
    ///
    /// Mutations only apply to inner changes, not removal or creation of components. If the component does not exist
    /// in the entity yet, the default one is inserted and used for the predicate.
    fn with_mutation_or_default<C: Component<Self> + Default, F: FnOnce(&mut C)>(mut self, f: F) -> Self {
        self.mutate_or_default(f);
        self
    }

    #[inline]
    /// Removes the given component for the given entity.
    fn with_removed<C: Component<Self>>(mut self) -> Self {
        self.remove::<C>();
        self
    }

    /// Depending on the current state of the component for the given entity, do some compelx operations.
    ///
    /// You must give a predicate that takes a `&mut Entity`, and returns a `ChangeComponent`.
    /// This is an enum that has four variants: one to change nothing, one to remove the component,
    /// one to replace (or add) a component, and another to mutate an already existing component.
    ///
    /// In all cases, the entity is returned. This is very useful if you have a component that is a "computed"
    /// value depending on other components.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let i: i32 = 4;
    /// let e = e.with_component_change(|e: &mut Entity| -> ChangeComponent<ComponentA> {
    ///     if i % 2 == 0 {
    ///         let beta = i + 1;
    ///         ChangeComponent::Mutate(Box::new(move |a: &mut ComponentA| {
    ///             a.alpha += beta as f32;
    ///         }))
    ///     } else {
    ///         ChangeComponent::NoChange
    ///     }
    /// });
    /// ```
    fn with_component_change<'a, C: Component<Self>, F: FnOnce(&mut Self) -> ChangeComponent<C>>(mut self, f: F) -> Self {
        match f(&mut self) {
            ChangeComponent::NoChange => self,
            ChangeComponent::Remove => self.with_removed::<C>(),
            ChangeComponent::Replace(c) => self.with(c),
            ChangeComponent::Mutate(f) => {
                if let Some(c) = self.get_mut::<C>() {
                    f(c)
                };
                self
            },
        }
    }

    #[inline]
    /// Peek the properties of the given component type, for the given entity, using the given predicate.
    ///
    /// You may chosse to return a custom type in your predicate. If the entity has the component,
    /// your value is returned, otherwise `None` is returned.
    fn peek<C: Component<Self>, O, F: FnOnce(&C) -> O>(&self, f: F) -> Option<O> {
        self.get::<C>().map(f)
    }

    #[inline]
    /// Mutate the properties of the given component type, for the given entity, using the given predicate.
    ///
    /// You may choose to return a custom type in your predicate. If the entity has the component,
    /// your value is returned, otherwise `None` is returned.
    fn mutate<C: Component<Self>, O, F: FnOnce(&mut C) -> O>(&mut self, f: F) -> Option<O> {
        self.get_mut::<C>().map(f)
    }

    #[inline]
    /// Returns true if the entity has the requested component type as an active component.
    fn has<C: Component<Self>>(&self) -> bool {
        C::get(self).is_some()
    }

    #[inline]
    fn get<C: Component<Self>>(&self) -> Option<&C> {
        C::get(self)
    }

    #[inline]
    fn get_mut<C: Component<Self>>(&mut self) -> Option<&mut C> {
        C::get_mut(self)
    }

    #[inline]
    /// Remove a component from the given entity.
    ///
    /// You MUST call `refresh(e_id)` if this entity is already part of the EntityList
    fn remove<C: Component<Self>>(&mut self) -> Option<Box<C>> {
        C::remove(self)
    }

    #[inline]
    /// Add a component to the given entity.
    ///
    /// You MUST call `refresh(e_id)` if this entity is already part of the EntityList
    fn add<C: Component<Self>>(&mut self, c: C) {
        c.set(self);
    }
}