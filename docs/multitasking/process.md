# Process 

## `Process` struct

```rust
struct Process {
    id:           usize,         // PID (monotonic, never reused)
    name:         [u8; 16],      // display name, NUL-padded
    mode:         Mode,          // Kernel (ring 0) or User (ring 3)
    status:       Status,        // see below
    last_rsp:     u64,           // saved kernel-stack RSP; the resume point
    kernel_stack: &'static [u8; 32768],  // per-slot kernel stack
    ports:        [Port; 1],     // IPC port (index 0 is the default)
    stack_top:    u64,           // initial user-space RSP
    cr3:          u64,           // physical address of P4 page table (0 = kernel CR3)
    sleep_until:  u64,           // PIT tick to wake from sleep (0 = not sleeping)
    waiter:       Option<Waiter>,// launcher parked until this process ends (fg)
}

struct Waiter {
    slot: usize,                 // launcher's slot
    id:   usize,                 // launcher's PID, to detect a recycled slot
}
```

## Status Transitions

```
              ┌────────────────────────────────────────────────┐
              │                                                ▼
  (new) ──► Ready ◄──── push_msg / wake ──── Blocked ──────► Ready
              │                                  ▲             ▲
              │  scheduler picks it              │             |
              ▼                                  │             |
           Running ──── blocking syscall ────────┘             |
              │                                                |
              ├────────────────────────────────────────────────┘
              |
              ├── kill()  ──────────────────────► Dead  (slot reaped, waiter woken)
              ├── crash() ──────────────────────► Crashed (stays, not scheduled, waiter woken)
              └── idle()  ──────────────────────► Idle  (stays, not scheduled)
```

| Status | Scheduled | Description |
|--------|-----------|-------------|
| `Ready` | yes | Runnable, waiting for its turn |
| `Running` | — | Currently executing on the CPU |
| `Blocked` | no | Waiting for a message or timer |
| `Idle` | no | Suspended: a kernel process, or a launcher parked on a foreground child |
| `Crashed` | no | Faulted; not rescheduled. Page tables and heap blocks are released, the slot is kept for `ts` until it is needed |
| `Dead` | no | Exited; page tables freed immediately in `kill()`, slot reclaimed on next scheduler pass |

## Privilege Modes

| `Mode` | GDT Ring | CS | SS |
|--------|---------|----|----|
| `Kernel` | 0 | `0x08` | `0x10` |
| `User` | 3 | `0x1b` | `0x23` |

Kernel processes use the same kernel stack as their run stack. User processes carry a separate user-space stack (whose top is stored in `stack_top`) plus a dedicated kernel stack that the CPU switches to on each syscall/interrupt via TSS `RSP0`.

## Initial Stack Frame

`new_process` builds the initial iretq frame on the kernel stack top:

```
 high address (kstack_top)
 ┌───────────────┐
 │ SS            │  ring-3: 0x23  / ring-0: 0x10
 │ RSP           │  user stack_top / kstack_top
 │ RFLAGS        │  0x202  (IF=1)
 │ CS            │  ring-3: 0x1b  / ring-0: 0x08
 │ RIP           │  entry point
 │ RAX..R15 (×15)│  zeroed general-purpose registers
 └───────────────┘  ← last_rsp points here
 low address
```

When the scheduler switches to a new process for the first time, it loads this RSP and the naked ISR exits via `iretq`, which pops RIP/CS/RFLAGS/RSP/SS and jumps to the entry point.

## Page Tables (CR3)

User processes get a dedicated P4 page table created by `elf::create_user_page_table`, which clones the kernel mappings and adds user-accessible entries for:

- `0x600_000–0x7FF_FFF` — ELF load region
- `0x7D0_000–0x8FF_FFF` — initial user stacks (one per slot, see [Memory Overview](../memory/overview.md#user-stack-tops-by-slot))
- `0xA00_000–0xAFF_FFF` — optional VGA window (mapped on demand by syscall `0x14`)
- `0xC00_000–0xFFF_FFF` — shared userland heap (4 MiB, mapped at `uheap::init`)

Kernel processes set `cr3 = 0`; the scheduler falls back to `KERNEL_CR3`.

### Page table reclamation

When `kill(pid)` is called, the scheduler calls `mem::pages::free_user_page_table(proc.cr3)` before marking the process `Dead`. This returns the P4, P3, and P2 pages (and any VGA P1 installed by `map_vram`) to the free list inside `PAGE_TABLE_POOL`, making them available for the next `create_user_page_table` call. `proc.cr3` is zeroed immediately after to prevent a double-free if `kill()` is called again for the same slot.

`crash(pid)` releases the page tables in the same way. The pool holds only 128 pages, so keeping a crashed process's three or four pages would eventually leave nothing for new processes. The slot itself is kept (and shown by `ts`, including the last `rip`) until `new_process` needs it.

If the process being released is the one currently running (it is inside the syscall that ends it), the CPU is first switched to `KERNEL_CR3`: a freed page is zeroed on its next allocation, which would otherwise unmap the code being executed.

---