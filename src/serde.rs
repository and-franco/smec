use crate::{EntityList, EntityRefBase};

use serde::de::{self, Deserialize, Deserializer, Visitor, SeqAccess, MapAccess};
use serde::ser::{Serialize, Serializer, SerializeStruct};

use crate::genarena::{GenArena, Entry};

impl<E> Serialize for EntityList<E>
where E: EntityRefBase, E::CS: Serialize, E::Naked: Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("EntityList", 4)?;
        let entries = self.entities.entries.iter().map(|e| {
            e.as_ref().map(|v| v.as_naked())
        }).collect::<Vec<_>>();
        state.serialize_field("entries", &entries)?;
        state.serialize_field("length", &self.entities.length)?;
        state.serialize_field("next_free", &self.entities.next_free)?;
        state.serialize_field("components_storage", unsafe { &*self.components_storage.get() })?;
        state.end()
    }
}

impl<'de, E> Deserialize<'de> for EntityList<E> where E: EntityRefBase, E::CS: Deserialize<'de>, E::Naked: Deserialize<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EntityListVisitor<E> { _phantom: std::marker::PhantomData<E> }
        impl<'de, E> Visitor<'de> for EntityListVisitor<E> where E: EntityRefBase, E::CS: Deserialize<'de>, E::Naked: Deserialize<'de> {
            type Value = EntityList<E>;
            
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("EntityList struct with 4 fields: entries, length, next_free, components_storage")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error> where V: SeqAccess<'de>,
            {
                let entries: Vec<Entry<E::Naked>> = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let length: usize = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let next_free: Option<usize> = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let components_storage: E::CS  = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                let components_storage = std::rc::Rc::new(std::cell::UnsafeCell::new(components_storage));
                let entries = entries.into_iter().map(|e| {
                    e.map(|v| E::from_naked(v, &components_storage))
                }).collect();
                Ok(EntityList::from_raw(
                    GenArena::from_raw(entries, length, next_free),
                    components_storage
                ))
            }

            fn visit_map<V>(self, _map: V) -> Result<Self::Value, V::Error> where V: MapAccess<'de>,
            {
                // see https://serde.rs/deserialize-struct.html to implement
                unimplemented!()
            }

        }

        deserializer.deserialize_struct(
            "EntityList",
            &["entries", "length", "next_free", "components_storage"],
            EntityListVisitor { _phantom: std::marker::PhantomData }
        )
        // let arena: GenArena<E> = Deserialize::deserialize(deserializer)?;
        // Ok(EntityList::from_arena(arena))
    }
}