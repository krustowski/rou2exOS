pub const MIN_HEAP_NODE_SIZE: usize = 0x10;

struct HeapNode {
    pub size: usize,
    pub status: HeapNodeStatus,
    pub previous: *mut HeapNode,
    pub next: *mut HeapNode,
}

#[derive(PartialEq)]
pub enum HeapNodeStatus {
    UNKNWON = 0x00,
    FREE,
    USED,
}

unsafe extern "C" {
    static __heap_start: u64;
    static __heap_end: u64;
}

static mut HEAP_PTR: usize = 0;

pub fn init() {
    unsafe { 
        let heap_start = &__heap_start as *const u64 as usize;
        let heap_end = &__heap_end as *const u64 as usize;

        HEAP_PTR = heap_start;

        let init_node = HeapNode {
            size: heap_end - heap_start,
            status: HeapNodeStatus::FREE,
            previous: 0 as *mut HeapNode,
            next: 0 as *mut HeapNode,
        };

        core::ptr::copy(&init_node, HEAP_PTR as *mut HeapNode, core::mem::size_of::<HeapNode>());

        rprint!("Kernel heap initialized: size: ");
        rprintn!(init_node.size);
        rprint!(" bytes.\n");
    }
}

unsafe fn merge() {
    let mut cur_node = HEAP_PTR as *mut HeapNode;

    while (*cur_node).status != HeapNodeStatus::FREE {
        if (*cur_node).previous > 0 as *mut HeapNode {
            cur_node = (*cur_node).previous;
            continue;
        }

        if (*cur_node).next > 0 as *mut HeapNode {
            cur_node = (*cur_node).next;
            continue;
        }

        debugln!("OOM: merge()")
    }

    if (*(*cur_node).previous).status == HeapNodeStatus::FREE {
        cur_node = (*cur_node).previous;

        let merged_node = HeapNode {
            size: (*cur_node).size + (*(*cur_node).next).size,
            status: HeapNodeStatus::FREE,
            previous: (*cur_node).previous,
            next: (*(*cur_node).next).next,
        };

        core::ptr::copy(&merged_node, cur_node, core::mem::size_of::<HeapNode>());
    }
}

unsafe fn split(node: *mut HeapNode, alloc_size: usize) {
    if (*node).size - MIN_HEAP_NODE_SIZE < alloc_size {
        // Do not split relatively small nodes.
        return;
    }

    let left_node = HeapNode {
        size: alloc_size,
        status: HeapNodeStatus::FREE,
        previous: (*node).previous,
        next: node.add(core::mem::size_of::<HeapNode>() + alloc_size),
    };

    let right_node = HeapNode {
        size: (*node).size - alloc_size,
        status: HeapNodeStatus::FREE,
        previous: &left_node as *const HeapNode as *mut HeapNode,
        next: (*node).next,
    };

    core::ptr::copy(&left_node, node, core::mem::size_of::<HeapNode>());
    core::ptr::copy(&right_node, left_node.next, core::mem::size_of::<HeapNode>());
}

pub unsafe fn alloc(alloc_size: usize) -> u64 {
    let mut cur_node = HEAP_PTR as *mut HeapNode;

    while (*cur_node).size < alloc_size {
        if (*cur_node).previous > 0 as *mut HeapNode && (*(*cur_node).previous).status == HeapNodeStatus::FREE {
            cur_node = (*cur_node).previous;
            continue;
        }

        if (*cur_node).next > 0 as *mut HeapNode && (*(*cur_node).next).status == HeapNodeStatus::FREE {
            cur_node = (*cur_node).next;
            continue;
        }

        merge();
    }

    if (*cur_node).size > alloc_size {
        split(cur_node, alloc_size);
    }

    (*cur_node).status = HeapNodeStatus::USED;

    // Return VAddr to allocated area
    (cur_node as *mut HeapNode as u64) + (core::mem::size_of::<HeapNode>() as u64)
}

pub unsafe fn free(vaddr: u64) {
    let mut cur_node = vaddr as *mut HeapNode;

    (*cur_node).status = HeapNodeStatus::FREE;
}
