use std::hash::BuildHasher;

use compact_str::CompactString;
use hashbrown::HashTable;
use hashbrown::hash_table::Entry;
use thin_vec::ThinVec;
use typed_index_collections::TiVec as IndexVec;

use crate::Symbol;

pub type Span = (usize, usize);

pub type Name = CompactString;

pub type Promise<T> = Option<T>;

// BX types

/// Represents the possible types in the language.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TyKind {
    /// Represents an integer type.
    Int,

    /// Represents a bool.
    Bool,

    /// Represents a void type, i.e. the unary type.
    Void,

    /// A type that was promised 3000 years ago.
    Promised,

    /// A placeholder type for a type that could not be computed.
    Error,

    /// Represents a pointer to a type, *T.
    Ptr(Ty),

    /// Represents a mutable pointer to a type, *mut T.
    MutPtr(Ty),

    /// Represents a reference to a type, &T.
    Ref(Ty),

    /// Represents a mutable reference to a type, &T.
    MutRef(Ty),
}

crate::index::index_type!(TypeIndex);

/// Represents an entry in the Type Interner.
/// Similar to how Symbol represents an underlying String.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Ty(TypeIndex);

const _: () = {
    assert!(size_of::<Option<Ty>>() == size_of::<Ty>());
};

#[allow(dead_code)]
impl Ty {
    pub const INT: Ty = Ty::new(0);
    pub const BOOL: Ty = Ty::new(1);
    pub const VOID: Ty = Ty::new(2);
    pub const PROMISED: Ty = Ty::new(3);
    pub const ERROR: Ty = Ty::new(4);

    #[must_use]
    #[inline(always)]
    pub const fn new(value: u32) -> Self {
        Self(TypeIndex::new(value))
    }

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
        for ty in [
            TyKind::Int,
            TyKind::Bool,
            TyKind::Void,
            TyKind::Promised,
            TyKind::Error,
        ] {
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
