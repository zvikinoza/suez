use std::collections::VecDeque;
use std::sync::{Arc, Condvar, LockResult, Mutex, MutexGuard};


struct Shared<T> {
    mu: Mutex<VecDeque<T>>,
    cond: Condvar,
}

pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) {
        let mut queue = self.shared.mu.lock().unwrap();
        queue.push_back(value);
        drop(queue);
        self.shared.cond.notify_one();
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        todo!()
    }
}

pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Receiver<T> {
    pub fn recv(&self) -> Option<T> {
        let mut queue = self.shared.mu.lock().unwrap();
        loop {
            match queue.pop_front() {
                None => {
                    queue = self.shared.cond.wait(queue).unwrap();
                }
                Some(value) => {
                    return Some(value);
                }
            }
        }
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Shared {
        mu: Mutex::new(VecDeque::new()),
        cond: Condvar::default()
    });
    let s = Sender {
        shared: Arc::clone(&shared),
    };
    let r = Receiver {
        shared: Arc::clone(&s.shared),
    };
    (s, r)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn init() {
        let (s, r) = channel::<()>();
    }

    #[test]
    fn send() {
        let (mut s, r) = channel();
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
        assert_eq!(r.recv(), None);
    }
}

