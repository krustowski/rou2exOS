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
}
```

## Status Transitions

```
            ┌──────────────────────────────────────────┐
            │                                          ▼
  (new) ──► Ready ◄──── push_msg / wake ──── Blocked ──────► Ready
              │                                  ▲
              │  scheduler picks it              │
              ▼                                  │
           Running ──── blocking syscall ────────┘
              │
              ├── kill()  ──────────────────────► Dead  (slot reaped)
              ├── crash() ──────────────────────► Crashed (stays, not scheduled)
              └── idle()  ──────────────────────► Idle  (stays, not scheduled)
```

| Status | Scheduled | Description |
|--------|-----------|-------------|
| `Ready` | yes | Runnable, waiting for its turn |
| `Running` | — | Currently executing on the CPU |
| `Blocked` | no | Waiting for a message or timer |
| `Idle` | no | Voluntarily suspended (kernel processes only) |
| `Crashed` | no | Faulted; not rescheduled but slot preserved for diagnostics |
| `Dead` | no | Exited; slot is reclaimed on next scheduler pass |

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
- `0x800_000–0x8FF_FFF` — user stacks (one 32 KiB stack per slot)
- `0xA00_000–0xAFF_FFF` — optional VGA window (mapped on demand by syscall `0x14`)
- `0xC00_000–0xFFF_FFF` — shared userland heap (4 MiB, mapped at `uheap::init`)

Kernel processes set `cr3 = 0`; the scheduler falls back to `KERNEL_CR3`.

---