use bevy::prelude::*;

macro_rules! mesh_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Default)]
        pub struct $name(pub u32);

        impl $name {
            pub const INVALID: Self = Self(u32::MAX);

            pub const fn new(index: u32) -> Self {
                Self(index)
            }

            pub const fn index(self) -> usize {
                self.0 as usize
            }

            pub const fn is_valid(self) -> bool {
                self.0 != u32::MAX
            }
        }
    };
}

mesh_id!(VertexId);
mesh_id!(HalfEdgeId);
mesh_id!(EdgeId);
mesh_id!(FaceId);
