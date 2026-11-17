use std::hash::BuildHasher;

use compact_str::CompactString;
use hashbrown::HashTable;
use hashbrown::hash_table::Entry;
use thin_vec::ThinVec;
use typed_index_collections::TiVec as IndexVec;

use crate::frontend::Symbol;

pub type Span = (usize, usize);

pub type Name = CompactString;

pub type Promise<T> = Option<T>;

// BX types

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TyKind {
    Int,
    Bool,
    Void,

    Ptr(Ty),
    MutPtr(Ty),

    Ref(Ty),
    MutRef(Ty),

    Promised,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ty(TypeIndex);

impl Ty {
    #[inline]
    pub fn intern(ty: TyKind) -> Self {
        crate::with_session_globals(|globals| globals.get_type_interner().intern_ty(ty))
    }

    #[inline]
    #[allow(dead_code)]
    pub fn kind(self) -> TyKind {
        crate::with_session_globals(|globals| globals.get_type_interner().get_ty(self))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeIndex(u32);

impl From<usize> for TypeIndex {
    #[inline]
    fn from(value: usize) -> Self {
        debug_assert!(value <= u32::MAX as usize);
        Self(value as u32)
    }
}

impl From<TypeIndex> for usize {
    #[inline]
    fn from(value: TypeIndex) -> Self {
        value.0 as usize
    }
}

pub struct TypeInterner(parking_lot::Mutex<InternerInner>);

struct InternerInner {
    types: IndexVec<TypeIndex, TyKind>,
    indices: HashTable<TypeIndex>,
    hasher: hashbrown::DefaultHashBuilder,
}

impl TypeInterner {
    pub fn new() -> Self {
        let inner = InternerInner {
            types: Default::default(),
            indices: Default::default(),
            hasher: Default::default(),
        };

        let interner = Self(parking_lot::Mutex::new(inner));
        for ty in [TyKind::Int, TyKind::Bool, TyKind::Void] {
            interner.intern_ty(ty);
        }
        interner
    }

    pub fn get_ty(&self, ty: Ty) -> TyKind {
        self.0.lock().types[ty.0]
    }

    pub fn intern_ty(&self, ty: TyKind) -> Ty {
        let mut inner = self.0.lock();

        let InternerInner {
            types,
            indices,
            hasher,
        } = &mut *inner;

        let hash = hasher.hash_one(ty);
        match indices.entry(
            hash,
            |&index| types[index] == ty,
            |&index| hasher.hash_one(types[index]),
        ) {
            Entry::Occupied(entry) => Ty(*entry.get()),
            Entry::Vacant(entry) => {
                let index = TypeIndex::from(types.len());
                types.push(ty);
                entry.insert(index);
                Ty(index)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ProcType {
    pub args_type: Option<ThinVec<(Symbol, TyKind)>>,
    pub return_type: Option<TyKind>,
}

#[derive(Clone)]
pub enum SemanticType {
    SimpleType(TyKind),
    ProcType(ProcType),
}
