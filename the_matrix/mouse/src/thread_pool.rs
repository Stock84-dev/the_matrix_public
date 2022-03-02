use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

use lazy_static::*;
use spmc::*;

pub struct ThreadPool {
    pool: Vec<Worker>,
    work_queue: Arc<Mutex<WorkQueue>>,
    max_pool_size: usize,
    rx: Receiver<WorkControl>,
    active_workers: Arc<Mutex<usize>>,
}

impl ThreadPool {
    pub fn new(initial_pool_size: usize, max_pool_size: usize) -> Self {
        assert_ne!(initial_pool_size, 0, "Min initial pool size is 1.");
        let (tx, rx) = channel();
        let work_queue = WorkQueue {
            urgent_queue: VecDeque::new(),
            queue: VecDeque::new(),
            sender: tx,
        };
        let work_queue = Arc::new(Mutex::new(work_queue));
        let mut pool = Vec::with_capacity(initial_pool_size);
        let active_workers = Arc::new(Mutex::new(0));
        for _ in 0..initial_pool_size {
            pool.push(Worker::new(
                active_workers.clone(),
                work_queue.clone(),
                rx.clone(),
            ));
        }
        Self {
            pool,
            work_queue,
            max_pool_size,
            rx,
            active_workers,
        }
    }

    pub fn enq<W: 'static + FnOnce() + Send>(&mut self, work: W) -> WorkHandle {
        let (handle, queue_len) = self.work_queue.lock().unwrap().enq(Box::new(work));
        self.maybe_add_worker(queue_len);
        handle
    }

    pub fn enq_urgent<W: 'static + FnOnce() + Send>(&mut self, work: W) -> WorkHandle {
        let (handle, queue_len) = self.work_queue.lock().unwrap().enq_urgent(Box::new(work));
        self.maybe_add_worker(queue_len);
        handle
    }

    fn maybe_add_worker(&mut self, queue_len: usize) {
        if self.pool.len() == self.max_pool_size {
            return;
        }
        let active_workers = { *self.active_workers.lock().unwrap() as i64 };
        if queue_len as i64 + active_workers - self.pool.len() as i64 > 0 {
            self.pool.push(Worker::new(
                self.active_workers.clone(),
                self.work_queue.clone(),
                self.rx.clone(),
            ));
        }
    }

    pub fn join(&mut self) {
        for _ in 0..self.pool.len() {
            self.work_queue
                .lock()
                .unwrap()
                .sender
                .send(WorkControl::Finish)
                .unwrap();
        }
        while let Some(worker) = self.pool.pop() {
            worker.join();
        }
    }
}

struct WorkQueue {
    urgent_queue: VecDeque<Box<dyn FnOnce() + Send>>,
    queue: VecDeque<Box<dyn FnOnce() + Send>>,
    sender: Sender<WorkControl>,
}

impl WorkQueue {
    fn get_work(&mut self) -> Box<dyn FnOnce() + Send> {
        return if !self.urgent_queue.is_empty() {
            self.urgent_queue.pop_front().unwrap()
        } else {
            self.queue.pop_front().unwrap()
        };
    }

    fn enq(&mut self, work_item: Box<dyn FnOnce() + Send>) -> (WorkHandle, usize) {
        self.queue.push_back(work_item);
        self.notify_workers()
    }

    fn enq_urgent(&mut self, work_item: Box<dyn FnOnce() + Send>) -> (WorkHandle, usize) {
        self.urgent_queue.push_back(work_item);
        self.notify_workers()
    }

    fn notify_workers(&mut self) -> (WorkHandle, usize) {
        let (tx, rx) = spmc::channel();
        self.sender.send(WorkControl::Continue(tx)).unwrap();
        (
            WorkHandle { rx },
            self.queue.len() + self.urgent_queue.len(),
        )
    }
}

struct Worker {
    join_handle: JoinHandle<()>,
}

impl Worker {
    fn new(
        active_counter: Arc<Mutex<usize>>,
        work_queue: Arc<Mutex<WorkQueue>>,
        rx: Receiver<WorkControl>,
    ) -> Self {
        Self {
            join_handle: Self::do_work(active_counter, work_queue, rx),
        }
    }

    fn join(self) {
        self.join_handle.join().ok().unwrap()
    }

    fn do_work(
        active_counter: Arc<Mutex<usize>>,
        work_queue: Arc<Mutex<WorkQueue>>,
        rx: Receiver<WorkControl>,
    ) -> JoinHandle<()> {
        thread::spawn(move || loop {
            let mut work_handle = match rx.recv().unwrap() {
                WorkControl::Continue(sender) => sender,
                WorkControl::Finish => break,
            };
            Self::change_active_workers_count(&active_counter, WorkerState::Active);
            let work = { work_queue.lock().unwrap().get_work() };
            work();
            Self::change_active_workers_count(&active_counter, WorkerState::Idle);
            // Notify that work is done.
            match work_handle.send(()) {
                _ => {}
            }
        })
    }

    fn change_active_workers_count(active_counter: &Arc<Mutex<usize>>, state: WorkerState) {
        if let WorkerState::Active = state {
            *active_counter.lock().unwrap() += 1;
        } else {
            *active_counter.lock().unwrap() -= 1;
        }
    }
}

pub struct WorkHandle {
    rx: Receiver<()>,
}

impl WorkHandle {
    pub fn finish(self) {
        self.rx.recv().unwrap()
    }
}

enum WorkControl {
    Continue(Sender<()>),
    Finish,
}

enum WorkerState {
    Active,
    Idle,
}

lazy_static! {
    static ref THREAD_POOL: Mutex<Option<ThreadPool>> = Mutex::new(None);
}

pub fn init_global(initial_pool_size: usize, max_pool_size: usize) {
    let mut a = THREAD_POOL.lock().unwrap();
    a.replace(ThreadPool::new(initial_pool_size, max_pool_size));
}

pub fn enq<W: 'static + FnOnce() + Send>(work: W) -> WorkHandle {
    THREAD_POOL.lock().unwrap().as_mut().unwrap().enq(work)
}

pub fn enq_urgent<W: 'static + FnOnce() + Send>(work: W) -> WorkHandle {
    THREAD_POOL
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .enq_urgent(work)
}

/*
use spmc::*;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::JoinHandle;

pub struct WorkPool<WI> {
    pool: Vec<JoinHandle<()>>,
    work_queue: Arc<Mutex<WorkQueue<WI>>>,
}

impl<WI: 'static + Send> WorkPool<WI> {
    pub fn new<W: 'static>(work_size: usize, work: W) -> Self
    where
        W: FnMut(WI) + Send + Clone,
    {
        let (sender, receiver) = channel();
        let work_queue = WorkQueue {
            queue: VecDeque::new(),
            sender,
        };
        let work_queue = Arc::new(Mutex::new(work_queue));
        let mut pool = Vec::with_capacity(work_size);
        for _ in 0..work_size {
            let worker = Worker::new(work_queue.clone(), receiver.clone());
            pool.push(worker.do_work(work.clone()));
        }
        Self { pool, work_queue }
    }

    pub fn enq(&mut self, work_item: WI) {
        self.work_queue.lock().unwrap().enq(work_item);
    }

    pub fn finish(&mut self) {
        for _ in 0..self.pool.len() {
            self.work_queue
                .lock()
                .unwrap()
                .sender
                .send(WorkControl::Finish)
                .unwrap();
        }
        while let Some(worker) = self.pool.pop() {
            worker.join().ok().unwrap();
        }
    }
}

struct WorkQueue<WI> {
    queue: VecDeque<WI>,
    sender: Sender<WorkControl>,
}

impl<WI> WorkQueue<WI> {
    fn get_work_item(&mut self) -> WI {
        self.queue.pop_front().unwrap()
    }

    fn enq(&mut self, work_item: WI) {
        self.queue.push_back(work_item);
        self.sender.send(WorkControl::Continue).unwrap();
    }
}

struct Worker<WI> {
    work_queue: Arc<Mutex<WorkQueue<WI>>>,
    receiver: Receiver<WorkControl>,
}

impl<WI: 'static + Send> Worker<WI> {
    fn new(work_queue: Arc<Mutex<WorkQueue<WI>>>, receiver: Receiver<WorkControl>) -> Self {
        Self {
            work_queue,
            receiver,
        }
    }

    fn do_work<W: 'static>(mut self, mut work: W) -> JoinHandle<()>
    where
        W: FnMut(WI) + Send,
    {
        thread::spawn(move || loop {
            match self.receiver.recv().unwrap() {
                WorkControl::Continue => {}
                WorkControl::Finish => break,
            }
            let work_item = { self.work_queue.lock().unwrap().get_work_item() };
            work(work_item);
        })
    }
}

#[derive(Clone)]
enum WorkControl {
    Continue,
    Finish,
}

*/
