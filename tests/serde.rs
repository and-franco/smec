#![cfg(feature = "use_serde")]

use serde::{
    Deserialize,
    Serialize,
};
use smec::{
    define_entity,
    EntityList,
    EntityBase, EntityRefBase, EntityOwnedBase
};

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct ComponentA {
    alpha: f32,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct ComponentB {
    beta: i32,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct CommonProp;

define_entity! {
    serde;
    #[derive(Debug)]
    pub struct Entity {
        props => {
            common: CommonProp,
        },
        components => {
            a => ComponentA,
            b => ComponentB,
        }
    }
}

fn are_equal(a: Option<&EntityRef>, b: Option<&EntityRef>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            dbg!(a.get::<ComponentA>() == b.get::<ComponentA>()) &&
            dbg!(a.get::<ComponentB>() == b.get::<ComponentB>()) &&
            a.common == b.common
        },
        (None, None) => true,
        _ => false
    }
}

#[test]
fn deserialized_have_same_values() {
    let mut entity_list: EntityList<EntityRef> = EntityList::new();

    let id_1 = entity_list.insert(
        Entity::new((CommonProp,))
            .with(ComponentA { alpha: 5.0 })
    );
    let id_2 = entity_list.insert(
        Entity::new((CommonProp,))
            .with(ComponentB { beta: 5 })
    );
    let id_3 = entity_list.insert(
        Entity::new((CommonProp,))
            .with(ComponentB { beta: 6 })
            .with(ComponentA { alpha: 6.0 })
    );
    let id_4 = entity_list.insert(
        Entity::new((CommonProp,))
            .with(ComponentB { beta: 6 })
            .with(ComponentA { alpha: 6.0 })
    );

    entity_list.remove(id_3);

    let bytes = bincode::serialize(&entity_list).expect("EntityList should be serializable");
    let deserialized_entity_list: EntityList<EntityRef> = bincode::deserialize(&bytes).expect("EntityList should be deserializable");

    // Check both arenas have the same values
    assert!(are_equal(entity_list.get(id_1), deserialized_entity_list.get(id_1)));
    assert!(are_equal(entity_list.get(id_2), deserialized_entity_list.get(id_2)));
    assert!(are_equal(entity_list.get(id_3), deserialized_entity_list.get(id_3)));
    assert!(are_equal(entity_list.get(id_4), deserialized_entity_list.get(id_4)));

    let all_entities: Vec<_> = entity_list.iter_all().map(|(i, _e)| i).collect();
    let only_comp_a: Vec<_> = entity_list.iter::<(ComponentA,)>().map(|(i, _e)| i).collect();
    let only_comp_b: Vec<_> = entity_list.iter::<(ComponentB,)>().map(|(i, _e)| i).collect();
    let comp_a_and_b: Vec<_> = entity_list.iter::<(ComponentA, ComponentB)>().map(|(i, _e)| i).collect();
    
    debug_assert_eq!(all_entities, &[id_1, id_2, id_4]);
    debug_assert_eq!(only_comp_a, &[id_1, id_4]);
    debug_assert_eq!(only_comp_b, &[id_2, id_4]);

    debug_assert_eq!(comp_a_and_b, &[id_4]);
}

#[test]
fn post_deserialize_same_behavior() {
    let mut entity_list: EntityList<EntityRef> = EntityList::new();

    let id_1 = entity_list.insert(
        Entity::new((CommonProp,))
            .with(ComponentA { alpha: 5.0 })
    );
    let id_2 = entity_list.insert(
        Entity::new((CommonProp,))
            .with(ComponentB { beta: 5 })
    );
    let id_3 = entity_list.insert(
        Entity::new((CommonProp,))
            .with(ComponentB { beta: 6 })
            .with(ComponentA { alpha: 6.0 })
    );
    let id_4 = entity_list.insert(
        Entity::new((CommonProp,))
            .with(ComponentB { beta: 6 })
            .with(ComponentA { alpha: 6.0 })
    );
    entity_list.remove(id_1);
    entity_list.remove(id_3);
    entity_list.remove(id_2);

    let bytes = bincode::serialize(&entity_list).expect("EntityList should be serializable");
    let mut deserialized_entity_list: EntityList<EntityRef> = bincode::deserialize(&bytes).expect("EntityList should be deserializable");

    let id_10 = entity_list.insert(
        Entity::new((CommonProp,))
            .with(ComponentB { beta: 5 })
    );
    let id_11 = entity_list.insert(
        Entity::new((CommonProp,))
            .with(ComponentA { alpha: 0.5 })
    );
    let id_20 = deserialized_entity_list.insert(
        Entity::new((CommonProp,))
            .with(ComponentB { beta: 5 })
    );
    let id_21 = deserialized_entity_list.insert(
        Entity::new((CommonProp,))
            .with(ComponentA { alpha: 0.5 })
    );
    assert_eq!(id_10, id_20);
    assert_eq!(id_11, id_21);
    let only_comp_a1: Vec<_> = entity_list.iter::<(ComponentA,)>().map(|(i, _e)| i).collect();
    let only_comp_b1: Vec<_> = entity_list.iter::<(ComponentB,)>().map(|(i, _e)| i).collect();
    let only_comp_a2: Vec<_> = deserialized_entity_list.iter::<(ComponentA,)>().map(|(i, _e)| i).collect();
    let only_comp_b2: Vec<_> = deserialized_entity_list.iter::<(ComponentB,)>().map(|(i, _e)| i).collect();
    assert_eq!(only_comp_a1, only_comp_a2);
    assert_eq!(only_comp_b1, only_comp_b2);
}