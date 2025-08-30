pub const MIN_HEAP_NODE_SIZE: usize = 0x10;

pub struct HeapNode {
    pub size: usize,
    pub status: HeapNodeStatus,
    pub previous: *mut HeapNode,
    pub next: *mut HeapNode,
}

#[derive(PartialEq)]
pub enum HeapNodeStatus {
    Unknown = 0x00,
    Free,
    Used,
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
            size: heap_end - heap_start - core::mem::size_of::<HeapNode>(),
            status: HeapNodeStatus::Free,
            previous: core::ptr::null_mut::<HeapNode>(),
            next: core::ptr::null_mut::<HeapNode>(),
        };

        core::ptr::copy(&init_node, HEAP_PTR as *mut HeapNode, 1);

        rprint!("Kernel heap initialized: size: ");
        rprintn!(init_node.size);
        rprint!(" bytes.\n");
    }
}

unsafe fn merge(mut node: *mut HeapNode) {
    while (*node).status != HeapNodeStatus::Free {
        if (*node).previous > core::ptr::null_mut::<HeapNode>() {
            node = (*node).previous;
            continue;
        }

        if (*node).next > core::ptr::null_mut::<HeapNode>() {
            node = (*node).next;
            continue;
        }

        rprint!("OOM: merge()\n");
    }

    if (*(*node).previous).status == HeapNodeStatus::Free {
        node = (*node).previous;

        let merged_node = HeapNode {
            size: (*node).size + (*(*node).next).size + core::mem::size_of::<HeapNode>(),
            status: HeapNodeStatus::Free,
            previous: (*node).previous,
            next: (*(*node).next).next,
        };

        rprint!("Merging nodes: total size: ");
        rprintn!(merged_node.size);
        rprint!(" bytes\n");

        core::ptr::copy(&merged_node, node, 1);

        let right_node = merged_node.next;
        (*right_node).previous = merged_node.previous;

        core::ptr::copy(right_node, merged_node.next, 1);

        HEAP_PTR = node as usize;
    }
}

unsafe fn split(node: *mut HeapNode, alloc_size: usize) {
    if (*node).size - MIN_HEAP_NODE_SIZE < alloc_size {
        // Do not split relatively small nodes.
        return;
    }

    let node_size = {
        if alloc_size < MIN_HEAP_NODE_SIZE {
            MIN_HEAP_NODE_SIZE
        } else {
            alloc_size
        }
    };

    let left_node = HeapNode {
        size: node_size,
        status: HeapNodeStatus::Free,
        previous: (*node).previous,
        next: node.add(core::mem::size_of::<HeapNode>() + node_size).addr() as *mut HeapNode,
    };

    let right_node = HeapNode {
        size: (*node).size - node_size - core::mem::size_of::<HeapNode>(),
        status: HeapNodeStatus::Free,
        previous: node,
        next: (*node).next,
    };

    rprint!("Splitting: left_node.size: ");
    rprintn!(left_node.size);
    rprint!(" bytes, right_node.size: ");
    rprintn!(right_node.size);
    rprint!(" bytes\n");

    core::ptr::copy(&right_node, left_node.next, 1);
    core::ptr::copy(&left_node, node, 1);
}

pub unsafe fn alloc(alloc_size: usize) -> u64 {
    let mut cur_node = HEAP_PTR as *mut HeapNode;

    let mut limit = 0;

    while (*cur_node).size < alloc_size {
        if (*cur_node).previous > core::ptr::null_mut::<HeapNode>() && (*(*cur_node).previous).status == HeapNodeStatus::Free {
            cur_node = (*cur_node).previous;
            continue;
        }

        if (*cur_node).next > core::ptr::null_mut::<HeapNode>() && (*(*cur_node).next).status == HeapNodeStatus::Free {
            cur_node = (*cur_node).next;
            continue;
        }

        rprint!("OOM: alloc()\n");
        limit += 1;

        if limit > 50 {
            return 0;
        }
    }

    if (*cur_node).size > alloc_size {
        split(cur_node, alloc_size);
    }

    // Reload the current node's metadata
    cur_node = HEAP_PTR as *mut HeapNode;

    // Zero the heap allocation
    cur_node.add(1).write_bytes(0, alloc_size);

    (*cur_node).status = HeapNodeStatus::Used;
    HEAP_PTR = (*cur_node).next as usize;

    // Return VAddr to allocated area
    //(cur_node as *mut HeapNode as u64) + (core::mem::size_of::<HeapNode>() as u64)
    cur_node as u64
}

pub unsafe fn free(vaddr: u64) {
    let cur_node = vaddr as *mut HeapNode;

    (*cur_node).status = HeapNodeStatus::Free;

    core::ptr::copy(cur_node, vaddr as *mut HeapNode, 1);

    merge(cur_node);
}
