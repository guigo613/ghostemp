use std::{
    thread,
    sync::{
        mpsc,
        Mutex,
        Arc
    }
};

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let mut workers = Vec::with_capacity(size);
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        for id in 1..=size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }

    pub fn execute<F> (&self, func: F) -> Result<(), String>
        where   F: FnOnce() + Send + 'static
    {
        let job = Box::new(func);

        self.sender.send(Message::NewJob(job)).map_err(|err| err.to_string())
    }
}

impl Clone for ThreadPool {
    fn clone(&self) -> Self {
        Self { workers: Vec::new(), sender: self.sender.clone() }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Encerrando");

        for _ in &self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            println!("Desligando ID: {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

unsafe impl Sync for ThreadPool {}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv().unwrap();

            match job {
                Message::NewJob(job) => {
                    job();
                },
                Message::Terminate => {
                    println!("Terminate ID: {}", id);

                    break;
                }
            }
        });
        
        Worker {
            id,
            thread: Some(thread),
        }
    }
}