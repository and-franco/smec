use super::*;

// Components storage, should be made of `Slab`s.
// 
// Note that Slab SHOULD be fine in our cases for ser/de, but be VERY careful.
// 
// Slab, when serialized, loose position of the "free" head they had. This means that after deserializing,
// the slabs will be inserted in a different order from the ones it was ser'd from.
//
// BUT as long as we do'nt directly iterate on the slab, we should be fine. If we do directly
// iterate on the slab at some point though, you will get weird shit...
pub trait ComponentsStorage: Clone {
    type Ref: EntityRefBase;
    fn new() -> Self;
}