use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

struct ChannelCtx<T> {
    queue: VecDeque<T>,
    n_senders: u32,
}

struct Shared<T> {
    mu: Mutex<ChannelCtx<T>>,
    cond: Condvar,
}

pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Sender<T> {
    pub fn send(&mut self, value: T) {
        let mut ctx = self.shared.mu.lock().unwrap();
        ctx.queue.push_back(value);
        drop(ctx);
        self.shared.cond.notify_one();
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let mut ctx = self.shared.mu.lock().unwrap();
        ctx.n_senders += 1;
        drop(ctx);
        Self {
            shared: Arc::clone(&self.shared)
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut ctx = self.shared.mu.lock().unwrap();
        ctx.n_senders -= 1;
        let is_last_samurai = ctx.n_senders == 0;
        drop(ctx);
        if is_last_samurai {
            self.shared.cond.notify_one();
        }
    }
}

pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
    buff: VecDeque<T>,
}

impl<T> Receiver<T> {
    pub fn recv(&mut self) -> Option<T> {
        if let Some(value) = self.buff.pop_front() {
            return Some(value);
        }

        let mut ctx = self.shared.mu.lock().unwrap();
        loop {
            match ctx.queue.pop_front() {
                None => {
                    if ctx.n_senders == 0 {
                        return None;
                    }
                    ctx = self.shared.cond.wait(ctx).unwrap();
                }
                Some(value) => {
                    if !ctx.queue.is_empty() {
                        std::mem::swap(&mut ctx.queue, &mut self.buff);
                    }
                    return Some(value);
                }
            }
        }
    }
}

impl<T> Iterator for Receiver<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.recv()
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Shared {
        mu: Mutex::new(ChannelCtx {
            queue: VecDeque::new(),
            n_senders: 1,
        }),
        cond: Condvar::new()
    });
    let s = Sender {
        shared: Arc::clone(&shared),
    };
    let r = Receiver {
        shared: Arc::clone(&shared),
        buff: VecDeque::new(),
    };
    (s, r)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn init() {
        let (_r, _c) = channel::<()>();
    }

    #[test]
    fn send() {
        let (mut s, _r) = channel();
        s.send(21);
    }

    #[test]
    fn recv() {
        let (mut s, mut r) = channel();
        s.send(21);
        assert_eq!(r.recv(), Some(21));
    }

    #[test]
    fn multiple_send() {
        let (mut s, mut r) = channel();
        s.send(21);
        s.send(22);
        let mut sc = s.clone();
        sc.send(27);
        assert_eq!(r.recv(), Some(21));
        assert_eq!(r.recv(), Some(22));
        assert_eq!(r.recv(), Some(27));
    }

    #[test]
    fn close_sending() {
        // should also close/drop recv
        let (mut s, mut r) = channel();
        s.send(21);
        drop(s);
        assert_eq!(r.recv(), Some(21));
        assert_eq!(r.recv(), None);
    }

    #[test]
    fn close_receiving() {
        let (mut s, r) = channel();
        drop(r);
        s.send(21);
    }
}

