const QUEUE_MSG_COUNT_MAX: usize = 10;

#[derive(Debug, Clone, Copy)]
pub struct Message {
    pub src_pid: usize,
    pub dst_pid: usize,
    pub port_id: usize,
    pub msg_len: usize,
    pub data: [u8; 512],
}

impl Message {
    pub fn new(
        port_id: usize,
        src_pid: usize,
        dst_pid: usize,
        msg_len: usize,
        data: [u8; 512],
    ) -> Self {
        Self {
            src_pid,
            dst_pid,
            port_id,
            msg_len,
            data,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Queue {
    buffer: [Option<Message>; QUEUE_MSG_COUNT_MAX],
    head: usize,
    tail: usize,
    msg_count: usize,
}

impl Queue {
    pub fn new() -> Queue {
        Queue {
            buffer: [None; QUEUE_MSG_COUNT_MAX],
            head: 0,
            tail: 0,
            msg_count: 0,
        }
    }

    pub fn push(&mut self, msg: Message) -> bool {
        if self.msg_count == QUEUE_MSG_COUNT_MAX {
            // The queue is full, or blocked
            return false;
        }

        self.buffer[self.tail] = Some(msg);

        // Update the counters and pointers
        self.tail = (self.tail + 1) % QUEUE_MSG_COUNT_MAX;
        self.msg_count += 1;

        true
    }

    pub fn pop(&mut self) -> Option<Message> {
        if self.msg_count == 0 {
            // The queue is empty
            return None;
        }

        // Fetch "the first" item from the queue and yeet its contents
        let msg = self.buffer[self.head].take();

        // Update the counters and pointers
        self.head = (self.head + 1) % QUEUE_MSG_COUNT_MAX;
        self.msg_count -= 1;

        msg
    }
}
