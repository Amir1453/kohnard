macro_rules! index_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        pub struct $name(::std::num::NonZeroU32);

        impl $name {
            #[must_use]
            #[inline(always)]
            pub const fn new(value: u32) -> Self {
                let stored = value.checked_add(1).expect("Index overflow");
                Self(::std::num::NonZeroU32::new(stored).unwrap())
            }
        }

        impl ::std::convert::From<u32> for $name {
            #[inline]
            fn from(value: u32) -> Self {
                Self::new(value)
            }
        }

        impl ::std::convert::From<$name> for u32 {
            #[inline]
            fn from(value: $name) -> Self {
                value.0.get() - 1
            }
        }

        impl ::std::convert::From<usize> for $name {
            #[inline]
            fn from(value: usize) -> Self {
                use ::std::convert::TryFrom;
                let stored = u32::try_from(value)
                    .ok()
                    .and_then(|n| n.checked_add(1))
                    .and_then(::std::num::NonZeroU32::new)
                    .expect("Index overflow");

                Self(stored)
            }
        }

        impl ::std::convert::From<$name> for usize {
            #[inline]
            fn from(value: $name) -> Self {
                (value.0.get() - 1) as usize
            }
        }

        const _: () = {
            assert!(
                ::std::mem::size_of::<::std::option::Option<$name>>()
                    == ::std::mem::size_of::<$name>()
            );
        };
    };
}

pub(crate) use index_type;
