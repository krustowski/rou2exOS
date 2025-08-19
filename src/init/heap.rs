use crate::mem::bump::{ALLOCATOR};
use core::ptr;

use super::result::InitResult;

pub fn print_result() -> InitResult {
    /*if !init_heap_allocator() {
        return InitResult::Failed;
    }*/

    crate::mem::heap::init();

    unsafe {
        let test_addr = crate::mem::heap::alloc(5) as usize;

        extern "C" {
            static __heap_start: u64;
            static __heap_end: u64;
        }

        let heap_start = &__heap_start as *const u64 as usize;
        let heap_end = &__heap_end as *const u64 as usize;

        if test_addr > heap_end || test_addr < heap_start {
            return InitResult::Failed;
        }

        crate::mem::heap::free(test_addr as u64);
    }

    InitResult::Passed
}

fn init_heap_allocator() -> bool {
    debugln!("Heap allocator init start");

    unsafe {
        unsafe extern "C" {
            static __heap_start: u8;
            static __heap_end: u8;
        }

        let heap_start = &__heap_start as *const u8 as usize;
        let heap_end = &__heap_end as *const u8 as usize;
        let heap_size = heap_end - heap_start;

        //#![allow(static_mut_refs)]
        let allocator_ptr = ptr::addr_of_mut!(ALLOCATOR);
        (*allocator_ptr).init(heap_start, heap_size);
    }

    true
}

