use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct FileWatcher {
    _thread: thread::JoinHandle<()>,
}

impl FileWatcher {
    pub fn new(path: PathBuf, callback: Box<dyn FnMut() + Send>) -> Self {
        let callback = Arc::new(Mutex::new(callback));
        let thread = thread::spawn(move || {
            let cb = callback.clone();
            let mut watcher = RecommendedWatcher::new(
                move |res: Result<Event, notify::Error>| {
                    if let Ok(event) = res
                        && matches!(event.kind, EventKind::Modify(_))
                        && let Ok(mut cb) = cb.lock()
                    {
                        cb();
                    }
                },
                Config::default(),
            )
            .expect("Could not create file watcher");
            watcher
                .watch(&path, RecursiveMode::NonRecursive)
                .expect("Could not watch file");
            loop {
                thread::park();
            }
        });
        Self { _thread: thread }
    }
}
