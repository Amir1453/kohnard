// How do I reason about how an arena works ?
//
// Technically, a Vector by itself is "kind of" an arena. Just with not bump allocation.
//
// Arena(Vec<T>), but this is not good, since if we return references to an object, when a reallaction
// hits, Vec will change its memory location.
//
// Arena(Vec<Box<T>>) This is better now, because even when an allocation hits, we do not invalidate
// the references. However, this is quite slow and doesnt keep the spirit of an arena.
//
// Arena(Vec<Box<ArenaChunk<T>>>) Like this, even when a reallocation hits the vector, the
// references will be valid, and we will have the benefit of using a bump allocator style
// ArenaChunk. Each chunk will be at least a page big, and will double with every allocation until
// we reach huge page size.
//
// We can instead store RefCell<Vec<ArenaChunk<T>>>, where RefCell provides interior mutability to
// Vec behind a reference to Arena, and Vec can directly store ArenaChunks, which will be each 24 bytes.
//
// The ArenaChunks will provide pointers into heap location, so a Vec reallocation containing
// ArenaChunks will not invalidate the references given.

// The following code was directly taken and/or inspired by the rustc/arena code.

#![allow(clippy::mut_from_ref)] // Arena allocators are one place where this pattern is fine.

use std::alloc::Layout;
use std::cell::{Cell, RefCell};
use std::mem::MaybeUninit;
use std::ptr::NonNull;

struct ArenaChunk {
    /// The raw storage for the arena chunk.
    /// Basically a pointer to a [u8].
    storage: NonNull<[MaybeUninit<u8>]>,
}

impl Drop for ArenaChunk {
    fn drop(&mut self) {
        unsafe { drop(Box::from_raw(self.storage.as_mut())) }
    }
}

impl ArenaChunk {
    #[inline]
    unsafe fn new(capacity: usize) -> ArenaChunk {
        let uninit_slice: Box<[MaybeUninit<u8>]> = Box::new_uninit_slice(capacity);
        let storage = NonNull::new(Box::into_raw(uninit_slice)).unwrap();

        ArenaChunk { storage }
    }

    // Returns a pointer to the first allocated object.
    #[inline]
    fn start(&mut self) -> *mut u8 {
        self.storage.as_ptr() as *mut u8
    }

    // Returns a pointer to the end of the allocated space.
    #[inline]
    fn end(&mut self) -> *mut u8 {
        unsafe { self.start().add(self.storage.len()) }
    }
}

// Page size in Linux is often 4KB, while huge pages are 2MB.
const PAGE: usize = 4096;
const HUGE_PAGE: usize = 2 * 1024 * 1024;

// Pointer alignment is common in compiler types, so keep `DroplessArena` aligned to them
// to optimize away alignment code.
const DROPLESS_ALIGNMENT: usize = align_of::<usize>();

pub struct DroplessArena {
    /// A pointer to the start of the free space.
    start: Cell<*mut u8>,

    /// A pointer to the end of free space.
    ///
    /// The allocation proceeds downwards from the end of the chunk towards the
    /// start. (This is slightly simpler and faster than allocating upwards,
    /// see <https://fitzgeraldnick.com/2019/11/01/always-bump-downwards.html>.)
    /// When this pointer crosses the start pointer, a new chunk is allocated.
    ///
    /// This is kept aligned to DROPLESS_ALIGNMENT.
    end: Cell<*mut u8>,

    /// A vector of arena chunks.
    chunks: RefCell<Vec<ArenaChunk>>,
}

impl Default for DroplessArena {
    #[inline]
    fn default() -> DroplessArena {
        DroplessArena {
            // We set both `start` and `end` to 0 so that the first call to
            // alloc() will trigger a grow().
            start: Cell::new(std::ptr::null_mut()),
            end: Cell::new(std::ptr::null_mut()),
            chunks: Default::default(),
        }
    }
}

impl DroplessArena {
    #[inline(never)]
    #[cold]
    fn grow(&self, layout: Layout) {
        // Add some padding so we can align `self.end` while
        // still fitting in a `layout` allocation.
        let additional = layout.size() + std::cmp::max(DROPLESS_ALIGNMENT, layout.align()) - 1;

        unsafe {
            let mut chunks = self.chunks.borrow_mut();
            let mut new_cap;
            if let Some(last_chunk) = chunks.last_mut() {
                // There is no need to update `last_chunk.entries` because that
                // field isn't used by `DroplessArena`.

                // If the previous chunk's len is less than HUGE_PAGE
                // bytes, then this chunk will be least double the previous
                // chunk's size.
                new_cap = last_chunk.storage.len().min(HUGE_PAGE / 2);
                new_cap *= 2;
            } else {
                new_cap = PAGE;
            }
            // Also ensure that this chunk can fit `additional`.
            new_cap = std::cmp::max(additional, new_cap);

            let chunk = chunks.push_mut(ArenaChunk::new(align_up(new_cap, PAGE)));
            self.start.set(chunk.start());

            // Align the end to DROPLESS_ALIGNMENT.
            let end = align_down(chunk.end().addr(), DROPLESS_ALIGNMENT);

            // Make sure we don't go past `start`. This should not happen since the allocation
            // should be at least DROPLESS_ALIGNMENT - 1 bytes.
            debug_assert!(chunk.start().addr() <= end);

            self.end.set(chunk.end().with_addr(end));
        }
    }

    #[inline]
    pub fn alloc_raw(&self, layout: Layout) -> *mut u8 {
        assert!(layout.size() != 0);

        // This loop executes once or twice: if allocation fails the first
        // time, the `grow` ensures it will succeed the second time.
        loop {
            let start = self.start.get().addr();
            let old_end = self.end.get();
            let end = old_end.addr();

            // Align allocated bytes so that `self.end` stays aligned to
            // DROPLESS_ALIGNMENT.
            let bytes = align_up(layout.size(), DROPLESS_ALIGNMENT);

            // Tell LLVM that `end` is aligned to DROPLESS_ALIGNMENT.
            unsafe { std::hint::assert_unchecked(end == align_down(end, DROPLESS_ALIGNMENT)) };

            if let Some(sub) = end.checked_sub(bytes) {
                let new_end = align_down(sub, layout.align());
                if start <= new_end {
                    let new_end = old_end.with_addr(new_end);
                    // `new_end` is aligned to DROPLESS_ALIGNMENT as `align_down`
                    // preserves alignment as both `end` and `bytes` are already
                    // aligned to DROPLESS_ALIGNMENT.
                    self.end.set(new_end);
                    return new_end;
                }
            }

            // No free space left. Allocate a new chunk to satisfy the request.
            // On failure the grow will panic or abort.
            self.grow(layout);
        }
    }

    /// Allocates a slice of objects that are copied into the `DroplessArena`, returning a mutable
    /// reference to it. Will panic if passed a zero-sized type.
    ///
    /// Panics:
    ///
    ///  - Zero-sized types
    ///  - Zero-length slices
    #[inline]
    pub fn alloc_slice<T>(&self, slice: &[T]) -> &mut [T]
    where
        T: Copy,
    {
        assert!(!std::mem::needs_drop::<T>());
        assert!(size_of::<T>() != 0);
        assert!(!slice.is_empty());

        let mem = self.alloc_raw(Layout::for_value::<[T]>(slice)) as *mut T;

        unsafe {
            mem.copy_from_nonoverlapping(slice.as_ptr(), slice.len());
            std::slice::from_raw_parts_mut(mem, slice.len())
        }
    }
}

#[inline(always)]
fn align_down(val: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    val & !(align - 1)
}

#[inline(always)]
fn align_up(val: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (val + align - 1) & !(align - 1)
}
