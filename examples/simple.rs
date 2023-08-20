#![allow(dead_code)]

use smec::{define_entity, EntityList, EntityOwnedBase, EntityBase};

#[derive(Clone, Debug)]
pub struct A {
    _n: i32
}

#[derive(Clone, Debug)]
pub struct B;

define_entity! { serde;
    pub struct Entity {
        props => { i: i32 }
        components => {
            a => A,
            b => B
        }
    }
}

fn main() {
    let mut list = EntityList::<EntityRef>::new();
    let id1 = list.insert(Entity::new((5i32,))
        .with(A { _n: 1 })
    );
    let id2 = list.insert(Entity::new((5i32,)));
    let id3 = list.insert(Entity::new((5i32,)).with(B));
    let e = list.get(id1).unwrap();
    if let Some(a) = e.get::<A>() {
        println!("{a:?}");
    }
    if let Some(b) = e.get::<B>() {
        println!("{b:?}");
    }
}