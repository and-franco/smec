
/// Macro to create an `Entity` type where this is called.
///
/// An entity has two main members:
///
/// * Properties, which are mandatory members on all your entities. Example: a position.
/// * Components, which are optional members taht may be added or removed at runtime. Examples:
/// a speed, a body, ...
///
/// The code below:
///
/// ```ignore
/// define_entity!{
///     pub struct Entity {
///         // if you have no props, use `props => {}` instead.
///         props => { a: A }
///         components => {
///             b => B,
///             c => C,
///         }
///     }
/// }
/// ```
///
/// will roughly generate the following code:
///
/// ```ignore
/// pub struct Entity {
///     pub a: A,
///     pub b: Option<Box<B>>,
///     pub c: Option<Box<C>>,
/// }
///
/// impl EntityBase for Entity { ... }
///
/// impl Component<Entity> for B { ... }
/// impl Component<Entity> for C { ... }
/// ```
///
/// ```rust
/// # use smec::define_entity;
/// define_entity! {
///     pub struct Entity {
///         props => {},
///         components => {}
///     }
/// }
/// ```
///
/// You can derive just as many things as you'd like with a regular struct. Only `Copy` is forbidden
/// if using components. Example:
///
/// ```ignore
/// define_entity! {
///     #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
///     pub struct Entity {
///         props => {},
///         components => {}
///     }
/// }

#[macro_export]
macro_rules! define_entity {
    (
        common;
        $vis:vis struct $entityname:ident {
            props => {
                $( $propname:ident : $propt:ty),* $(,)*
            } $(,)?
            components => {
                $( $componentname:ident => $componenttype:ty ),* $(,)*
            } $(,)?
        }
    ) => {
        $crate::paste::paste! {

        impl Clone for [<$entityname ComponentsStorage>] {
            fn clone(&self) -> Self {
                Self {
                    $(
                        $componentname: self.$componentname.clone(),
                    )*
                }
            }

            fn clone_from(&mut self, other: &Self) {
                $(
                self.$componentname.clone_from(&other.$componentname);
                )*
            }
        }
        }

        $(
            impl smec::Component<$entityname> for $componenttype {
                #[inline]
                fn set(self, entity: &mut $entityname) {
                    entity.$componentname = Some(Box::new(self))
                }

                #[inline]
                fn get(entity: &$entityname) -> Option<&$componenttype> {
                    entity.$componentname.as_ref().map(|s| &**s)
                }

                #[inline]
                fn get_mut(entity: &mut $entityname) -> Option<&mut $componenttype> {
                    entity.$componentname.as_mut().map(|s| &mut **s)
                }

                #[inline]
                fn remove(entity: &mut $entityname) -> Option<Box<$componenttype>> {
                    entity.$componentname.take()
                }

                #[inline]
                fn peek<O, F: FnOnce(&Self) -> O>(entity: &$entityname, f: F) -> Option<O> {
                    entity.$componentname.as_ref().map(|c| &**c).map(f)
                }

                #[inline]
                fn update<O, F: FnOnce(&mut Self) -> O>(entity: &mut $entityname, f: F) -> Option<O> {
                    entity.$componentname.as_mut().map(|c| &mut **c).map(f)
                }
            }

            $crate::paste::paste! {
            impl smec::Component<[<$entityname Ref>]> for $componenttype {
                fn set(self, entity: &mut EntityRef) {
                    let current = entity.$componentname;
                    if let Some(storage) = entity.components_storage.upgrade() {
                        unsafe {
                            if let Some(current) = current {
                                if let Some(old) = (*storage.get()).$componentname.get_mut(current)  {
                                    *old = self;
                                    return;
                                }
                            }
                            entity.$componentname = Some((*storage.get()).$componentname.insert(self));
                        }
                    } else {
                        unreachable!()
                    }
                }

                fn get(entity: &EntityRef) -> Option<&$componenttype> {
                    if let Some(current) = entity.$componentname {
                        if let Some(storage) = entity.components_storage.upgrade() {
                            unsafe {
                                (*storage.get()).$componentname.get(current)
                            }
                        } else {
                            unreachable!()
                        }
                    } else {
                        None
                    }
                }

                fn get_mut(entity: &mut EntityRef) -> Option<&mut $componenttype> {
                    if let Some(current) = entity.$componentname {
                        if let Some(storage) = entity.components_storage.upgrade() {
                            // SAFETY: a bit more debatable, if we have 2 EntityRef mutable at the same time this is a violation
                            // of safety !!BUT!! this is technically not possible because all EntityRef are stored in the arena,
                            // and there is no get2(..) method in there.
                            // we also cannot (or should not if this is not implemented yet) be able to construct EntityRef ourselves
                            unsafe {
                                (*storage.get()).$componentname.get_mut(current)
                            }
                        } else {
                            unreachable!()
                        }
                    } else {
                        None
                    }
                }

                fn remove(entity: &mut EntityRef) -> Option<Box<$componenttype>> {
                    if let Some(current) = entity.$componentname.take() {
                        if let Some(storage) = entity.components_storage.upgrade() {
                            // SAFETY: in theory we only access the component of the entity from the storage,
                            // so this is safe?
                            unsafe {
                                Some(Box::new((*storage.get()).$componentname.remove(current)))
                            }
                        } else {
                            unreachable!()
                        }
                    } else {
                        None
                    }
                }

                fn peek<O, F: FnOnce(&Self) -> O>(entity: &EntityRef, f: F) -> Option<O> {
                    if let Some(current) = entity.$componentname {
                        if let Some(storage) = entity.components_storage.upgrade() {
                            // SAFETY: in theory we only access the component of the entity from the storage,
                            // so this is safe?
                            unsafe {
                                if let Some(c) = (*storage.get()).$componentname.get(current) {
                                    Some(f(c))
                                } else {
                                    None
                                }
                            }
                        } else {
                            unreachable!()
                        }
                    } else {
                        None
                    }
                }

                fn update<O, F: FnOnce(&mut Self) -> O>(entity: &mut EntityRef, f: F) -> Option<O> {
                    if let Some(current) = entity.$componentname {
                        if let Some(storage) = entity.components_storage.upgrade() {
                            // SAFETY: in theory we only access the component of the entity from the storage,
                            // so this is safe?
                            unsafe { 
                                if let Some(c) = (*storage.get()).$componentname.get_mut(current) {
                                    Some(f(c))
                                } else {
                                    None
                                }
                            }
                        } else {
                            unreachable!()
                        }
                    } else {
                        None
                    }
                }
            }
            
            impl smec::RefComponent<[<$entityname Ref>]> for $componenttype {
                #[inline]
                fn get_single_cs(cs: &[<$entityname ComponentsStorage>]) -> &$crate::slab::Slab<Self> {
                    &cs.$componentname
                }

                #[inline]
                fn get_cs_id(entity: &[<$entityname Ref>]) -> Option<usize> {
                    entity.$componentname
                }
            }
            }
        )*

        impl Clone for $entityname {
            fn clone(&self) -> Self {
                Self {
                    $(
                        $propname: self.$propname.clone(),
                    )*
                    $(
                        $componentname: self.$componentname.clone(),
                    )*
                }
            }

            fn clone_from(&mut self, other: &Self) {
                $(
                    self.$propname.clone_from(&other.$propname);
                )*
                $(
                    self.$componentname.clone_from(&other.$componentname);
                )*
            }
        }

        $crate::paste::paste! {
        impl smec::EntityBase for [<$entityname Ref>] {
            fn for_each_active_component(&self, mut f: impl FnMut(std::any::TypeId)) {
                $(
                    if self.$componentname.is_some() {
                        f(std::any::TypeId::of::< $componenttype >())
                    };
                )*
            }

            fn for_each_component(&self, mut f: impl FnMut(std::any::TypeId, bool)) {
                $(
                    f(std::any::TypeId::of::< $componenttype >(), self.$componentname.is_some());
                )*
            }

            fn for_all_components(mut f: impl FnMut(std::any::TypeId)) {
                $(
                    f(std::any::TypeId::of::< $componenttype >());
                )*
            }
        }

        impl smec::EntityRefBase for [<$entityname Ref>] {
            type CS = [<$entityname ComponentsStorage>];
            type Owned = $entityname;
            type Naked = [<$entityname RefNaked>];

            fn from_owned(mut owned: Self::Owned, cs: &std::rc::Rc<std::cell::UnsafeCell<Self::CS>>) -> Self {
                let weak = std::rc::Rc::downgrade(cs);
                let borrowed_cell = cs.get();
                Self {
                    $(
                        $propname : owned.$propname,
                    )*
                    $(
                        $componentname : owned.$componentname.take().map(|c| {
                            unsafe { (*borrowed_cell).$componentname.insert(*c) }
                        }),
                    )*
                    components_storage: weak,
                }
            }

            fn to_owned(self, cs: &mut Self::CS) -> Self::Owned {
                Self::Owned {
                    $(
                        $propname : self.$propname,
                    )*
                    $(
                        $componentname : self.$componentname.map(|c_id| {
                            if let Some(cs) = self.components_storage.upgrade() {
                                unsafe {
                                    Box::new((*cs.get()).$componentname.remove(c_id))
                                }
                            } else {
                                unreachable!()
                            }
                        }),
                    )*
                }
            }

            fn from_naked(naked: Self::Naked, cs: &std::rc::Rc<std::cell::UnsafeCell<Self::CS>>) -> Self {
                Self {
                    $(
                        $propname : naked.$propname,
                    )*
                    $(
                        $componentname : naked.$componentname,
                    )*
                    components_storage: std::rc::Rc::downgrade(cs)
                }
            }

            fn as_naked(&self) -> Self::Naked {
                Self::Naked {
                    $(
                        $propname : self.$propname.clone(),
                    )*
                    $(
                        $componentname : self.$componentname,
                    )*
                }
            }

            fn set_cs(&mut self, cs: std::rc::Weak<std::cell::UnsafeCell<Self::CS>>) {
                self.components_storage = cs;
            }
        }
        
        impl smec::ComponentsStorage for [<$entityname ComponentsStorage>] {
            type Ref = [<$entityname Ref>];

            fn new() -> Self {
                Self {
                    $(
                        $componentname: $crate::slab::Slab::new(),
                    )*
                }
            }
        }
        }

        impl smec::EntityBase for $entityname {
            fn for_each_active_component(&self, mut f: impl FnMut(std::any::TypeId)) {
                $(
                    if self.$componentname.is_some() {
                        f(std::any::TypeId::of::< $componenttype >())
                    };
                )*
            }

            fn for_each_component(&self, mut f: impl FnMut(std::any::TypeId, bool)) {
                $(
                    f(std::any::TypeId::of::< $componenttype >(), self.$componentname.is_some());
                )*
            }

            fn for_all_components(mut f: impl FnMut(std::any::TypeId)) {
                $(
                    f(std::any::TypeId::of::< $componenttype >());
                )*
            }
        }

        impl smec::EntityOwnedBase for $entityname {
            type CreationParams = ( $( $propt ,)* );

            fn new( ( $( $propname ,)* ) : ( $( $propt ,)*) ) -> Self {
                $entityname {
                    $(
                        $propname: $propname,
                    )*
                    $(
                        $componentname: None,
                    )*
                }
            }
        }
    };
    (   
        serde;
        $(#[derive( $( $derivety:path ),* ) ])?
        $vis:vis struct $entityname:ident {
            props => {
                $( $propname:ident : $propt:ty),* $(,)*
            } $(,)?
            components => {
                $( $componentname:ident => $componenttype:ty ),* $(,)*
            } $(,)?
        }
    ) => {
        $crate::paste::paste!{
        #[derive($crate::serde::Serialize, $crate::serde::Deserialize)]
        $(#[derive( $( $derivety ),* )])?
        $vis struct $entityname {
            $(
                pub $propname : $propt,
            )*
            $(
                pub $componentname: Option<Box<$componenttype>>,
            )*
        }

        #[derive(Clone)]
        $vis struct [<$entityname Ref>] {
            $(
                pub $propname : $propt,
            )*
            $(
                pub $componentname: Option<usize>,
            )*
            components_storage: std::rc::Weak<::std::cell::UnsafeCell<[<$entityname ComponentsStorage>]>>
        }

        #[derive(Clone)]
        #[derive($crate::serde::Serialize, $crate::serde::Deserialize)]
        $vis struct [<$entityname RefNaked>] {
            $(
                pub $propname : $propt,
            )*
            $(
                pub $componentname: Option<usize>,
            )*
        }

        #[derive($crate::serde::Serialize, $crate::serde::Deserialize)]
        $vis struct [<$entityname ComponentsStorage>] {
            $(
                $componentname: $crate::slab::Slab<$componenttype>,
            )*
        }
        }

        smec::define_entity! {
            common;
            $vis struct $entityname {
                props => {
                    $(
                        $propname: $propt,
                    )*
                },
                components => {
                    $(
                        $componentname => $componenttype,
                    )*
                }
            }
        }
    };
    (
        $(#[derive( $( $derivety:path ),* ) ])?
        $vis:vis struct $entityname:ident {
            props => {
                $( $propname:ident : $propt:ty),* $(,)*
            } $(,)?
            components => {
                $( $componentname:ident => $componenttype:ty ),* $(,)*
            } $(,)?
        }
    ) => {
        $crate::paste::paste! {
        $(#[derive( $( $derivety ),* )])?
        $vis struct $entityname {
            $(
                pub $propname : $propt,
            )*
            $(
                pub $componentname: Option<Box<$componenttype>>,
            )*
        }

        #[derive(Clone)]
        $vis struct [<$entityname Ref>] {
            $(
                pub $propname : $propt,
            )*
            $(
                pub $componentname: Option<usize>,
            )*
            components_storage: std::rc::Weak<::std::cell::UnsafeCell<[<$entityname ComponentsStorage>]>>
        }

        #[derive(Clone)]
        $vis struct [<$entityname RefNaked>] {
            $(
                pub $propname : $propt,
            )*
            $(
                pub $componentname: Option<usize>,
            )*
        }

        $vis struct [<$entityname ComponentsStorage>] {
            $(
                $componentname: $crate::slab::Slab<$componenttype>,
            )*
        }
        }

        smec::define_entity! {
            common;
            $vis struct $entityname {
                props => {
                    $(
                        $propname: $propt,
                    )*
                },
                components => {
                    $(
                        $componentname => $componenttype,
                    )*
                }
            }
        }
    }
}
