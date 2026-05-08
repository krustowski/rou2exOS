# Scheduler

rou2exOS uses a cooperative/preemptive round-robin scheduler driven by the PIT (Programmable Interval Timer) at 100 Hz (IRQ 0 → interrupt `0x20`). Each PIT tick fires `scheduler_schedule`, which saves the interrupted process's RSP, picks the next runnable process, loads its RSP, and switches CR3 to its page table. The running process can also voluntarily yield by entering a blocking syscall.

## Data Structures

```rust
struct Scheduler {
    processes: [Option<Process>; MAX_PROCESSES],  // up to 10 slots
    current_pid: usize,
    next_free_pid: usize,
}
```

The scheduler is a global `spin::Mutex<Scheduler>`, accessed as `SCHEDULER`. All mutations go through `try_lock`; if the lock is already held (e.g., a PIT tick fires during a syscall that already holds it), the tick is silently skipped and the current process continues.

## Scheduling Algorithm

On every PIT tick:

1. Any `Blocked` process whose `sleep_until` ≤ current tick is transitioned to `Ready`.
2. A linear scan from `(current_pid + 1) % len` forward finds the next slot that is not `None`, `Blocked`, `Crashed`, or `Idle`. If no such slot exists, the interrupted RSP is returned unchanged (current process keeps running).
3. The outgoing process's RSP is saved into `process.last_rsp`. Its status is set to `Ready` (unless it is already `Blocked`, `Crashed`, `Dead`, or `Idle`).
4. `Dead` processes are reaped (slot set to `None`) both eagerly on detection and lazily at the start of the next tick.
5. The incoming process's status is set to `Running`. The TSS `RSP0` field is updated to the top of the incoming process's kernel stack so that the next ring-3 → ring-0 transition lands on the right stack.
6. CR3 is written with the incoming process's page table address, flushing the TLB.
7. The incoming `last_rsp` is returned to the naked ISR, which uses it as the new stack pointer for `iretq`.

## Tick and Sleep

The tick counter is maintained by `crate::time::acpi::tick()` / `get_tick_count()`, incremented once per PIT interrupt. It has 10 ms resolution.

`sleep_current(until_tick)` marks the calling process `Blocked`, stores `until_tick` in `process.sleep_until`, and executes `hlt`. The scheduler wakes the process automatically on the first tick at or after `until_tick`. The `hlt` also yields host CPU time to QEMU's event loop so PS/2 input is not starved.

## PID vs Slot

PIDs are assigned by a monotonically-incrementing counter (`next_free_pid`). Slots (`processes[0..MAX_PROCESSES]`) are reused — a slot can hold processes with different PIDs over time. The kernel stack is pinned to a slot (not a PID) via `KSTACK_POOL[slot]` so each slot always has a dedicated 32 KiB kernel stack.

## Special Processes

| Slot | Name | Mode | Purpose |
|------|------|------|---------|
| 0 | `kmain` | Kernel | Sentinel/idle — absorbs the boot RSP on the first PIT tick, then loops on `hlt` |
| 1 | `init_rc` | Kernel | Reads `INIT.RC` from FAT12 root and dispatches each line through the shell command handler; exits when done |
| 2 | `clock` | Kernel | Renders a live HH:MM:SS clock in the top-left VGA text buffer corner |
| 3 | `shell` | Kernel | Kernel interactive shell; keyboard input loop; PID stored in `SHELL_PID` |
| 4+ | *(userland)* | User | ELF processes spawned via `run_elf` / syscall `0x2A` |

---

## Limits

| Resource | Value |
|----------|-------|
| Max concurrent processes | 10 |
| Kernel stack per process | 32 KiB |
| Message queue depth | 10 messages |
| `MSG_BUF` payload size | 512 bytes |
| Pipe buffer size | ~14 KiB |
| Scheduler tick rate | 100 Hz (10 ms resolution) |
| PID namespace | monotonic `usize`, never reused |
