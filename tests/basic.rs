use smec::{
    define_entity,
    EntityList,
    EntityBase,
    EntityRefBase,
    EntityOwnedBase,
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ComponentA {
    alpha: f32,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ComponentB {
    beta: i32,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ComponentC {
    ceta: u32,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct CommonProp;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct AgeProp {
    age: u32,
}

define_entity! { 
    pub struct Entity {
        props => {
            common: CommonProp,
            age: AgeProp,
        },
        components => {
            a => ComponentA,
            b => ComponentB,
            c => ComponentC,
        }
    }
}

#[test]
fn entity_ops() {
    let mut entity_list: EntityList<EntityRef> = EntityList::new();

    let id_1 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 5 }))
            .with(ComponentA { alpha: 5.0 })
    );
    let id_2 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentB { beta: 5 })
    );
    let e1 = entity_list.get(id_1).unwrap();
    let x = e1.get::<ComponentA>().unwrap();
    debug_assert_eq!(x.alpha, 5.0);
    debug_assert_eq!(e1.get::<ComponentB>(), None);

    let e2 = entity_list.get_mut(id_2).unwrap();
    let x = e2.get_mut::<ComponentB>().unwrap();
    x.beta += 1;

    debug_assert!(e2.has::<ComponentB>());
    debug_assert!(! e2.has::<ComponentA>());
    e2.mutate(|c: &mut ComponentB| { c.beta += 1 });

    let v = e2.peek(|c: &ComponentB| c.beta );
    debug_assert_eq!(v, Some(7));

    e2.remove::<ComponentB>();
    e2.add(ComponentC { ceta: 7 });

    debug_assert_eq!(e2.get::<ComponentB>(), None);
    debug_assert!(e2.get::<ComponentC>().is_some());
}

#[test]
fn entity_with_ops() {
    let e = Entity::new((CommonProp, AgeProp { age: 5 }))
            .with(ComponentA { alpha: 5.0 });
    let e = e.with_removed::<ComponentA>();
    debug_assert_eq!(e.get::<ComponentA>(), None);

    let e = e.with(ComponentB { beta: 5 });
    debug_assert_eq!(e.get::<ComponentB>().clone(), Some(&ComponentB { beta: 5 }));

    let e = e.with_mutation(|c: &mut ComponentB| {
        c.beta += 1;
    });

    debug_assert_eq!(e.get::<ComponentB>().clone(), Some(&ComponentB { beta: 6 }));
}

#[test]
fn entity_with_component_change() {
    use smec::ChangeComponent;

    let e = Entity::new((CommonProp, AgeProp { age: 5 }))
            .with(ComponentA { alpha: 5.0 })
            .with(ComponentB { beta: 5 });
    let e = e.with_component_change(|e: &mut Entity| -> ChangeComponent<ComponentA> {
        if let Some(_) = e.get::<ComponentB>() {
            ChangeComponent::Remove
        } else {
            ChangeComponent::NoChange
        }
    });

    debug_assert_eq!(e.get::<ComponentA>(), None);

    let e = e.with_component_change(|e: &mut Entity| -> ChangeComponent<ComponentA> {
        if let Some(ComponentB { beta }) = e.get::<ComponentB>() {
            ChangeComponent::Replace(ComponentA { alpha: 5.0 + (*beta as f32) })
        } else {
            ChangeComponent::NoChange
        }
    });

    debug_assert_eq!(e.get::<ComponentA>(), Some(&ComponentA { alpha: 10.0 }));

    let e = e.with_component_change(|e: &mut Entity| -> ChangeComponent<ComponentA> {
        if let Some(ComponentB { beta }) = e.get::<ComponentB>() {
            let beta = *beta;
            ChangeComponent::Mutate(Box::new(move |a: &mut ComponentA| {
                a.alpha += beta as f32;
            }))
        } else {
            ChangeComponent::NoChange
        }
    });

    debug_assert_eq!(e.get::<ComponentA>(), Some(&ComponentA { alpha: 15.0 }));
    
    let e = e.with_component_change(|_: &mut Entity| -> ChangeComponent<ComponentA> {
        ChangeComponent::NoChange
    });

    debug_assert_eq!(e.get::<ComponentA>(), Some(&ComponentA { alpha: 15.0 }));
}

#[test]
/// Tests that properties are available
fn entity_prop_ops() {
    let e = Entity::new((CommonProp, AgeProp { age: 5 }))
            .with(ComponentA { alpha: 5.0 });
    debug_assert_eq!(e.common, CommonProp);
    debug_assert_eq!(e.age, AgeProp { age: 5 });
}

#[test]
/// Tests immutable iteration, and also that bitsets can be added after adding entities.
/// Also test that iteration works even with a partial coverage of entities.
fn iter() {
    let mut entity_list: EntityList<EntityRef> = EntityList::new();

    let id_1 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 5 }))
            .with(ComponentA { alpha: 5.0 })
    );
    let id_2 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 1 }))
            .with(ComponentB { beta: 5 })
    );
    let id_3 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentB { beta: 6 })
            .with(ComponentA { alpha: 6.0 })
    );
    let id_4 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentC { ceta: 6 })
    );
    let id_5 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentA { alpha: 6.0 })
            .with(ComponentB { beta: 6 })
            .with(ComponentC { ceta: 6 })
    );
    let id_6 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentB { beta: 6 })
            .with(ComponentC { ceta: 6 })
    );
    let id_7 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentA { alpha: 6.0 })
            .with(ComponentB { beta: 6 })
    );
    let id_8 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentA { alpha: 6.0 })
            .with(ComponentB { beta: 6 })
            .with(ComponentC { ceta: 6 })
    );

    entity_list.remove(id_5);
    entity_list.remove(id_7);

    let all_entities: Vec<_> = entity_list.iter_all().map(|(i, _e)| i).collect();
    let only_comp_a: Vec<_> = entity_list.iter::<(ComponentA,)>().map(|(i, _e)| i).collect();
    let only_comp_b: Vec<_> = entity_list.iter::<(ComponentB,)>().map(|(i, _e)| i).collect();
    let only_comp_c: Vec<_> = entity_list.iter::<(ComponentC,)>().map(|(i, _e)| i).collect();
    let comp_a_and_b: Vec<_> = entity_list.iter::<(ComponentA, ComponentB)>().map(|(i, _e)| i).collect();
    let comp_a_and_c: Vec<_> = entity_list.iter::<(ComponentA, ComponentC)>().map(|(i, _e)| i).collect();
    let comp_b_and_c: Vec<_> = entity_list.iter::<(ComponentB, ComponentC)>().map(|(i, _e)| i).collect();
    let comp_all: Vec<_> = entity_list.iter::<(ComponentB, ComponentC, ComponentA)>().map(|(i, _e)| i).collect();

    debug_assert_eq!(all_entities, &[id_1, id_2, id_3, id_4, id_6, id_8]);

    debug_assert_eq!(only_comp_a, &[id_1, id_3, id_8]);
    debug_assert_eq!(only_comp_b, &[id_2, id_3, id_6, id_8]);
    debug_assert_eq!(only_comp_c, &[id_4, id_6, id_8]);

    debug_assert_eq!(comp_a_and_b, &[id_3, id_8]);
    debug_assert_eq!(comp_a_and_c, &[id_8]);
    debug_assert_eq!(comp_b_and_c, &[id_6, id_8]);
    
    debug_assert_eq!(comp_all, &[id_8]);
}

#[test]
/// Tests mutable iteration, and also that bitsets can be added before adding entities.
fn iter_mut() {
    let mut entity_list: EntityList<EntityRef> = EntityList::new();

    let id_1 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 5 }))
            .with(ComponentA { alpha: 5.0 })
    );
    let id_2 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 1 }))
            .with(ComponentB { beta: 5 })
    );
    let id_3 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentB { beta: 6 })
            .with(ComponentA { alpha: 6.0 })
    );
    let id_4 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentC { ceta: 6 })
    );
    let id_5 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentB { beta: 6 })
            .with(ComponentC { ceta: 6 })
    );
    let id_6 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentA { alpha: 6.0 })
            .with(ComponentB { beta: 6 })
            .with(ComponentC { ceta: 6 })
    );

    let all_entities: Vec<_> = entity_list.iter_all_mut().map(|(i, _e)| i).collect();
    let only_comp_a: Vec<_> = entity_list.iter_mut::<(ComponentA,)>().map(|(i, _e)| i).collect();
    let only_comp_b: Vec<_> = entity_list.iter_mut::<(ComponentB,)>().map(|(i, _e)| i).collect();
    let only_comp_c: Vec<_> = entity_list.iter_mut::<(ComponentC,)>().map(|(i, _e)| i).collect();
    let comp_a_and_b: Vec<_> = entity_list.iter_mut::<(ComponentA, ComponentB)>().map(|(i, _e)| i).collect();
    let comp_a_and_c: Vec<_> = entity_list.iter_mut::<(ComponentA, ComponentC)>().map(|(i, _e)| i).collect();
    let comp_b_and_c: Vec<_> = entity_list.iter_mut::<(ComponentB, ComponentC)>().map(|(i, _e)| i).collect();
    let comp_all: Vec<_> = entity_list.iter_mut::<(ComponentB, ComponentC, ComponentA)>().map(|(i, _e)| i).collect();

    debug_assert_eq!(all_entities, &[id_1, id_2, id_3, id_4, id_5, id_6]);

    debug_assert_eq!(only_comp_a, &[id_1, id_3, id_6]);
    debug_assert_eq!(only_comp_b, &[id_2, id_3, id_5, id_6]);
    debug_assert_eq!(only_comp_c, &[id_4, id_5, id_6]);

    debug_assert_eq!(comp_a_and_b, &[id_3, id_6]);
    debug_assert_eq!(comp_a_and_c, &[id_6]);
    debug_assert_eq!(comp_b_and_c, &[id_5, id_6]);
    
    debug_assert_eq!(comp_all, &[id_6]);


    let mut v: Vec<_> = Vec::new();

    for el in entity_list.iter_mut::<(ComponentA,)>().map(|(_i, e)| e) {
        v.push(el);
    }

    // // this is commented, but it should NOT compile if you uncomment it. If there is one day a
    // // way to create #[compile_fail] tests, then we could put that here.
    // for el in entity_list.iter_mut::<(ComponentB,)>().map(|(_i, e)| e) {
    //     v.push(el);
    // }
}

#[test]
/// Tests mutable iteration, and also that bitsets can be added before adding entities.
fn iter_refresh() {
    let mut entity_list: EntityList<EntityRef> = EntityList::new();

    let id_1 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 5 }))
            .with(ComponentA { alpha: 5.0 })
    );
    let id_2 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 1 }))
            .with(ComponentB { beta: 5 })
    );
    let id_3 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentB { beta: 6 })
            .with(ComponentA { alpha: 6.0 })
    );
    let id_4 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentC { ceta: 6 })
    );
    let id_5 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentB { beta: 6 })
            .with(ComponentC { ceta: 6 })
    );
    let id_6 = entity_list.insert(
        Entity::new((CommonProp, AgeProp { age: 6 }))
            .with(ComponentA { alpha: 6.0 })
            .with(ComponentB { beta: 6 })
            .with(ComponentC { ceta: 6 })
    );

    if let Some(e) = entity_list.get_mut(id_6) {
        dbg!(e.a, e.b, e.c);
        e.remove::<ComponentB>();
        dbg!(e.a, e.b, e.c);
    }
    entity_list.refresh(id_6);
    if let Some(e) = entity_list.get_mut(id_2) {
        e.add::<ComponentA>(ComponentA { alpha: 4.0 });
    }
    entity_list.refresh(id_2);

    let all_entities: Vec<_> = entity_list.iter_all_mut().map(|(i, _e)| i).collect();
    let only_comp_a: Vec<_> = entity_list.iter_mut::<(ComponentA,)>().map(|(i, _e)| i).collect();
    let only_comp_b: Vec<_> = entity_list.iter_mut::<(ComponentB,)>().map(|(i, _e)| i).collect();
    let only_comp_c: Vec<_> = entity_list.iter_mut::<(ComponentC,)>().map(|(i, _e)| i).collect();

    debug_assert_eq!(all_entities, &[id_1, id_2, id_3, id_4, id_5, id_6]);

    debug_assert_eq!(only_comp_a, &[id_1, id_2, id_3, id_6]);
    debug_assert_eq!(only_comp_b, &[id_2, id_3, id_5]);
    debug_assert_eq!(only_comp_c, &[id_4, id_5, id_6]);
}