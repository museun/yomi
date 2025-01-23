use std::{
    path::{Path, PathBuf},
    thread::JoinHandle,
    time::{Duration, SystemTime},
};

pub struct Watcher {
    _handle: JoinHandle<()>,
    events: flume::Receiver<()>,
}

impl Watcher {
    pub fn new(script: impl Into<PathBuf>) -> Self {
        let (tx, events) = flume::unbounded();

        let _handle = std::thread::spawn({
            let script = script.into();
            move || Self::watch(script, tx)
        });

        Self { _handle, events }
    }

    pub fn next_event(&self) -> &flume::Receiver<()> {
        &self.events
    }

    fn last_modified(path: &Path) -> Option<SystemTime> {
        std::fs::read_dir(path)
            .unwrap()
            .flatten()
            .filter_map(|f| {
                let md = f.metadata().ok()?;
                md.is_file().then_some(md.modified().ok()?)
            })
            .max_by_key(|x| *x)
    }

    // TODO this has to watch the directory for changes, to reload the main
    // script. not just the main script
    fn watch(path: PathBuf, tx: flume::Sender<()>) {
        let mut last = SystemTime::now();

        loop {
            let Some((elapsed, next)) = Self::last_modified(&path)
                .and_then(|md| md.duration_since(last).ok().map(|e| (e, md)))
            else {
                // this waiting for a second is rather unfortunate.
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue;
            };

            if elapsed >= Duration::from_millis(100) {
                last = next;
                if tx.send(()).is_err() {
                    return;
                }
            }
        }
    }
}
