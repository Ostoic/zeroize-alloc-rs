#![no_std]
#![feature(allocator_api)]

use core::alloc::{GlobalAlloc, Allocator, Layout};

pub struct ZeroizingGlobalAllocator<Alloc: GlobalAlloc>(pub Alloc);

pub struct ZeroizingAllocator<Alloc: Allocator>(pub Alloc);

#[cfg_attr(feature = "aggressive-inline", inline)]
unsafe fn zero(ptr: *mut u8, size: usize) {
    for i in 0..size {
        core::ptr::write_volatile(ptr.offset(i as isize), 0);
    }
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

unsafe impl<A> Allocator for ZeroizingAllocator<A>
where
    A: Allocator
{
    #[cfg_attr(feature = "aggressive-inline", inline(always))]
    fn allocate(&self, layout: Layout) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        self.0.allocate(layout)
    }

    #[cfg_attr(feature = "aggressive-inline", inline(always))]
    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: Layout) {
        zero(ptr.as_ptr(), layout.size());
        // #[cfg(not(test))]
        self.0.deallocate(ptr.clone(), layout);
    }
}

unsafe impl<A> GlobalAlloc for ZeroizingGlobalAllocator<A>
where
    A: GlobalAlloc,
{
    #[cfg_attr(feature = "aggressive-inline", inline(always))]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0.alloc(layout)
    }

    #[cfg_attr(feature = "aggressive-inline", inline(always))]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        zero(ptr, layout.size());
        #[cfg(not(test))]
        self.0.dealloc(ptr, layout);
    }

    #[cfg_attr(feature = "aggressive-inline", inline(always))]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.0.alloc_zeroed(layout)
    }
}

#[cfg(test)]
mod test {
    extern crate std;
    use std::vec::Vec;

    #[global_allocator]
    static ALLOC: super::ZeroizingGlobalAllocator<std::alloc::System> =
        super::ZeroizingGlobalAllocator(std::alloc::System);

    #[test]
    fn test_static() {
        let mut a = Vec::with_capacity(2);
        a.push(0xde);
        a.push(0xad);
        let ptr1: *const u8 = &a[0];
        a.push(0xbe);
        a.push(0xef);
        let ptr2: *const u8 = &a[0];
        assert_eq!(&[0xde, 0xad, 0xbe, 0xef], &a[..]);
        assert_eq!(unsafe { ptr1.as_ref() }, Some(&0));
        drop(a);
        assert_eq!(unsafe { ptr2.as_ref() }, Some(&0));
    }

    quickcheck::quickcheck! {
        fn prop(v1: Vec<u8>, v2: Vec<u8>) -> bool {
            let mut v1 = v1;
            if v1.len() == 0 || v2.len() == 0 {
                return true;
            }
            let ptr1: *const u8 = &v1[0];
            v1.shrink_to_fit();
            let ptr2: *const u8 = &v2[0];
            v1.extend(v2);
            let ptr3: *const u8 = &v1[0];
            assert_eq!(unsafe { ptr1.as_ref() }, Some(&0));
            assert_eq!(unsafe { ptr2.as_ref() }, Some(&0));
            drop(v1);
            assert_eq!(unsafe { ptr3.as_ref() }, Some(&0));
            true
        }
    }
}
