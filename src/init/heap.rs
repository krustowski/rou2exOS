use crate::mem::bump::{ALLOCATOR};
use core::ptr;

//use super::result::InitResult;

/*pub fn print_result() -> InitResult {
    /*if !init_heap_allocator() {
      return InitResult::Failed;
      }*/

    unsafe { 
        crate::mem::pmm::pmm_init();
    }

    crate::mem::heap::init();

    unsafe {
        for _ in 0..3 {
            let test_addr0 = crate::mem::heap::alloc(5) as usize;
            let test_addr1 = crate::mem::heap::alloc(50) as usize;
            let test_addr2 = crate::mem::heap::alloc(500) as usize;

            extern "C" {
                static __heap_start: u64;
                static __heap_end: u64;
            }

            let heap_start = &__heap_start as *const u64 as usize;
            let heap_end = &__heap_end as *const u64 as usize;

            if test_addr0 > heap_end || test_addr0 < heap_start {
                return InitResult::Failed;
            }

            if test_addr1 > heap_end || test_addr1 < heap_start {
                return InitResult::Failed;
            }

            if test_addr2 > heap_end || test_addr2 < heap_start {
                return InitResult::Failed;
            }

            let test_node0 = test_addr0 as *mut crate::mem::heap::HeapNode;
            let test_node1 = test_addr1 as *mut crate::mem::heap::HeapNode;
            let test_node2 = test_addr2 as *mut crate::mem::heap::HeapNode;

            rprint!("Test heap allocation: node size: ");
            rprintn!((*test_node0).size as u64);
            rprint!(" bytes\n");

            rprint!("Test heap allocation: node size: ");
            rprintn!((*test_node1).size as u64);
            rprint!(" bytes\n");

            rprint!("Test heap allocation: node size: ");
            rprintn!((*test_node2).size as u64);
            rprint!(" bytes\n");

            crate::mem::heap::free(test_addr0 as u64);
            crate::mem::heap::free(test_addr1 as u64);
            crate::mem::heap::free(test_addr2 as u64);
        }
    }

    InitResult::Passed
} */

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

