//! Backend calls run off the UI thread. They read config files and run `hyprctl`, `nmcli`,
//! `wpctl` and the like, which can take from milliseconds to a second; the window keeps drawing
//! meanwhile. Each job's answer lands through `poll` on the UI thread.
//!
//! There are two lanes. Reads (`run`) each get a thread and may overlap. Writes (`write`)
//! queue on one long-lived thread and run one at a time, in the order they were asked for, so
//! read-modify-write edits of the same file can't interleave and `wpctl` volumes arrive in
//! order. Work answers `Result<T, String>`; a panic lands as an `Err`, so every `land` runs and
//! can clear its busy state.
//!
//! Jobs only run once the native window is ready (`go_live` from `App::on_ready`). Headless
//! runs, tests and snapshots never touch the system: they record which jobs were asked for, and
//! scenes supply the data.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rune::{Tx, Waker};

use crate::app::State;

type Landing = Box<dyn FnOnce(&mut State, &mut Tx) + Send>;

/// A job, ready to go: called with `None` it does its work, with `Some(error)` it skips the
/// work (it couldn't be started) and lands that error. Either way it lands exactly once.
type Task = Box<dyn FnOnce(Option<String>) + Send>;

pub struct Jobs {
    send: Sender<Landing>,
    inbox: Receiver<Landing>,
    waker: Option<Waker>,
    live: bool,
    /// The write lane, started with the first write.
    writer: Option<Sender<Task>>,
    /// Jobs asked for while not live, in order (for tests).
    pub skipped: Vec<&'static str>,
}

impl Jobs {
    pub fn new() -> Jobs {
        let (send, inbox) = channel();
        Jobs {
            send,
            inbox,
            waker: None,
            live: false,
            writer: None,
            skipped: Vec::new(),
        }
    }

    pub fn set_waker(&mut self, waker: Waker) {
        self.waker = Some(waker);
    }

    pub fn go_live(&mut self) {
        self.live = true;
    }

    /// Reads: runs `work` on its own worker thread, then `land` with its answer on the UI
    /// thread. Reads may run alongside each other and alongside a write.
    pub fn run<T: Send + 'static>(
        &mut self,
        name: &'static str,
        work: impl FnOnce() -> Result<T, String> + Send + 'static,
        land: impl FnOnce(&mut State, Result<T, String>, &mut Tx) + Send + 'static,
    ) {
        if !self.live {
            self.skipped.push(name);
            return;
        }
        let task = self.task(name, work, land);
        // The thread takes the task from the slot; if it never starts, it's still here.
        let slot = Arc::new(Mutex::new(Some(task)));
        let theirs = Arc::clone(&slot);
        let spawned = std::thread::Builder::new()
            .name(format!("job-{name}"))
            .spawn(move || {
                let task = theirs.lock().ok().and_then(|mut t| t.take());
                if let Some(task) = task {
                    task(None);
                }
            });
        if let Err(e) = spawned {
            let task = slot.lock().ok().and_then(|mut t| t.take());
            if let Some(task) = task {
                task(Some(format!("Couldn't start {name}: {e}")));
            }
        }
    }

    /// Changes to files or the system: `work` queues behind every write asked for before it
    /// and runs on the one writer thread, then `land` gets its answer on the UI thread.
    pub fn write<T: Send + 'static>(
        &mut self,
        name: &'static str,
        work: impl FnOnce() -> Result<T, String> + Send + 'static,
        land: impl FnOnce(&mut State, Result<T, String>, &mut Tx) + Send + 'static,
    ) {
        if !self.live {
            self.skipped.push(name);
            return;
        }
        let task = self.task(name, work, land);
        let refused = match self.writer() {
            Ok(writer) => writer.send(task).err().map(|e| e.0),
            Err(e) => {
                task(Some(format!("Couldn't start {name}: {e}")));
                return;
            }
        };
        if let Some(task) = refused {
            // The writer is gone (it never panics, so this is shutdown); don't queue again.
            self.writer = None;
            task(Some(format!("Couldn't start {name}: the writer stopped")));
        }
    }

    /// A source that keeps producing (a socket, a watcher) on a thread of its own: `source`
    /// gets an `emit` that lands each value through `land` on the UI thread, and answers false
    /// once the app is gone, when the source should return.
    pub fn stream<T: Send + 'static>(
        &mut self,
        name: &'static str,
        source: impl FnOnce(&mut dyn FnMut(T) -> bool) + Send + 'static,
        land: impl Fn(&mut State, T, &mut Tx) + Send + Clone + 'static,
    ) {
        if !self.live {
            self.skipped.push(name);
            return;
        }
        let send = self.send.clone();
        let waker = self.waker.clone();
        let spawned = std::thread::Builder::new()
            .name(format!("stream-{name}"))
            .spawn(move || {
                let mut emit = |value: T| {
                    let land = land.clone();
                    let landing: Landing =
                        Box::new(move |s: &mut State, tx: &mut Tx| land(s, value, tx));
                    let delivered = send.send(landing).is_ok();
                    if delivered && let Some(w) = &waker {
                        w();
                    }
                    delivered
                };
                source(&mut emit);
            });
        if let Err(e) = spawned {
            eprintln!("hyprconfig: couldn't start {name}: {e}");
        }
    }

    /// Watches something cheap to check (file times, say): `probe` runs every `every`, and
    /// once its answer has changed and then held for one more check, `land` runs on the UI
    /// thread. The thread ends when the app does.
    pub fn watch<F: PartialEq + Send + 'static>(
        &mut self,
        name: &'static str,
        every: Duration,
        mut probe: impl FnMut() -> F + Send + 'static,
        land: impl Fn(&mut State, &mut Tx) + Send + Clone + 'static,
    ) {
        self.stream(
            name,
            move |emit| {
                let mut settle = Settle::new(probe());
                loop {
                    std::thread::sleep(every);
                    if settle.check(probe()) && !emit(()) {
                        return;
                    }
                }
            },
            move |s, (), tx| land(s, tx),
        );
    }

    fn writer(&mut self) -> std::io::Result<&Sender<Task>> {
        if self.writer.is_none() {
            let (send, queue) = channel::<Task>();
            std::thread::Builder::new()
                .name("job-writer".into())
                .spawn(move || {
                    // Tasks catch their own panics, so one failing write doesn't stop the rest.
                    for task in queue {
                        task(None);
                    }
                })?;
            self.writer = Some(send);
        }
        Ok(self.writer.as_ref().expect("started above"))
    }

    fn task<T: Send + 'static>(
        &self,
        name: &'static str,
        work: impl FnOnce() -> Result<T, String> + Send + 'static,
        land: impl FnOnce(&mut State, Result<T, String>, &mut Tx) + Send + 'static,
    ) -> Task {
        let send = self.send.clone();
        let waker = self.waker.clone();
        Box::new(move |refused: Option<String>| {
            let out = match refused {
                Some(e) => Err(e),
                None => guarded(name, work),
            };
            let landing: Landing = Box::new(move |s: &mut State, tx: &mut Tx| land(s, out, tx));
            if send.send(landing).is_ok()
                && let Some(w) = waker
            {
                w();
            }
        })
    }

    /// Lands every finished job; true when any did.
    pub fn drain(s: &mut State, tx: &mut Tx) -> bool {
        let mut landed = false;
        while let Ok(landing) = s.jobs.inbox.try_recv() {
            landing(s, tx);
            landed = true;
        }
        landed
    }
}

/// Debounces a watched value: a change counts once it has held for one more check, so a
/// burst of writes (a file replaced in steps, several files saved together) fires once.
pub struct Settle<F> {
    seen: F,
    pending: bool,
}

impl<F: PartialEq> Settle<F> {
    pub fn new(seen: F) -> Settle<F> {
        Settle {
            seen,
            pending: false,
        }
    }

    /// True when the value changed and has now held still.
    pub fn check(&mut self, now: F) -> bool {
        if now != self.seen {
            self.seen = now;
            self.pending = true;
            false
        } else {
            std::mem::take(&mut self.pending)
        }
    }
}

/// Runs `work`, turning a panic into an error naming the job.
fn guarded<T>(name: &str, work: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(work))
        .unwrap_or_else(|p| Err(format!("{name} crashed: {}", panic_message(p.as_ref()))))
}

fn panic_message(p: &(dyn Any + Send)) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = p.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_owned()
    }
}

/// Attempt numbers are unique across every `Remote`, so an answer to a request made before a
/// `Remote` was replaced can't match one made after.
fn next_attempt() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Hold {
    #[default]
    None,
    /// A write to what the data describes is in flight.
    Writing,
    /// The write landed; the data is stale until the reload after it lands.
    Reloading,
}

/// Data loaded from the system: the latest answer, whether a newer request is out, and the
/// last error. A reload keeps showing the previous data until its answer lands, and answers to
/// superseded requests are dropped.
pub struct Remote<T> {
    pub data: Option<T>,
    pub loading: bool,
    pub error: Option<String>,
    attempt: u64,
    hold: Hold,
}

impl<T> Default for Remote<T> {
    fn default() -> Self {
        Remote {
            data: None,
            loading: false,
            error: None,
            attempt: 0,
            hold: Hold::None,
        }
    }
}

impl<T> Remote<T> {
    /// Already loaded (scenes and tests).
    pub fn ready(data: T) -> Remote<T> {
        Remote {
            data: Some(data),
            ..Remote::default()
        }
    }

    pub fn failed(error: impl Into<String>) -> Remote<T> {
        Remote {
            error: Some(error.into()),
            ..Remote::default()
        }
    }

    /// Starts a request; pass the attempt to `land`.
    pub fn begin(&mut self) -> u64 {
        self.attempt = next_attempt();
        self.loading = true;
        self.error = None;
        self.attempt
    }

    /// Applies the answer to `attempt`; false (and no change) when a newer request superseded it.
    pub fn land(&mut self, attempt: u64, result: Result<T, String>) -> bool {
        if attempt != self.attempt {
            return false;
        }
        self.loading = false;
        match result {
            Ok(data) => self.data = Some(data),
            Err(e) => self.error = Some(e),
        }
        if self.hold == Hold::Reloading {
            self.hold = Hold::None;
        }
        true
    }

    /// Nothing to show yet: never answered, or reloading with nothing from before.
    pub fn first_load(&self) -> bool {
        self.data.is_none() && (self.loading || self.error.is_none())
    }

    /// A write is changing what this data describes. Until `release`, or the reload after
    /// `written` lands, rows mustn't be acted on by their position.
    pub fn hold(&mut self) {
        self.hold = Hold::Writing;
    }

    /// The write failed, so the data still describes the file.
    pub fn release(&mut self) {
        self.hold = Hold::None;
    }

    /// The write succeeded: settled once the next accepted answer lands. Reload right after.
    pub fn written(&mut self) {
        self.hold = Hold::Reloading;
    }

    /// Whether a write or its reload is outstanding.
    pub fn settling(&self) -> bool {
        self.hold != Hold::None
    }
}

#[cfg(test)]
mod tests {
    use super::{Remote, Settle, guarded};

    #[test]
    fn a_watched_change_fires_once_it_settles() {
        let mut w = Settle::new(1);
        assert!(!w.check(1), "nothing changed");
        assert!(!w.check(2), "changed, not settled yet");
        assert!(!w.check(3), "still changing");
        assert!(w.check(3), "held for a check");
        assert!(!w.check(3), "fires once");
    }

    #[test]
    fn a_superseded_answer_is_dropped() {
        let mut r: Remote<u32> = Remote::default();
        let first = r.begin();
        let second = r.begin();
        assert!(!r.land(first, Ok(1)));
        assert!(r.loading);
        assert!(r.land(second, Ok(2)));
        assert_eq!(r.data, Some(2));
        assert!(!r.loading);
    }

    #[test]
    fn reloading_keeps_the_old_data_and_clears_the_error() {
        let mut r = Remote::ready(5);
        let a = r.begin();
        assert_eq!(r.data, Some(5));
        assert!(!r.first_load());
        r.land(a, Err("boom".into()));
        assert_eq!(r.data, Some(5));
        assert_eq!(r.error.as_deref(), Some("boom"));
        r.begin();
        assert!(r.error.is_none());
    }

    #[test]
    fn first_load_until_answered() {
        let mut r: Remote<u32> = Remote::default();
        assert!(r.first_load());
        let a = r.begin();
        assert!(r.first_load());
        r.land(a, Err("no".into()));
        assert!(!r.first_load());
    }

    #[test]
    fn an_answer_from_before_a_remote_was_replaced_is_dropped() {
        let mut r: Remote<u32> = Remote::default();
        let old = r.begin();
        // The page throws the Remote away (say, after a reload) and asks again.
        r = Remote::default();
        let new = r.begin();
        assert_ne!(old, new);
        assert!(!r.land(old, Ok(1)));
        assert_eq!(r.data, None);
        assert!(r.land(new, Ok(2)));
        assert_eq!(r.data, Some(2));
    }

    #[test]
    fn a_write_settles_when_the_reload_after_it_lands() {
        let mut r = Remote::ready(vec![1, 2, 3]);
        let before = r.begin();
        r.hold();
        assert!(r.settling());
        // A reload that started before the write doesn't settle it.
        assert!(r.land(before, Ok(vec![1, 2, 3])));
        assert!(r.settling());
        r.written();
        let after = r.begin();
        assert!(r.settling());
        assert!(r.land(after, Ok(vec![1, 3])));
        assert!(!r.settling());

        r.hold();
        r.release();
        assert!(!r.settling());
    }

    #[test]
    fn a_panicking_job_answers_with_an_error() {
        let out: Result<u32, String> = guarded("save-keybind", || panic!("index out of range"));
        assert_eq!(
            out,
            Err("save-keybind crashed: index out of range".to_owned())
        );
        let out: Result<u32, String> = guarded("x", || panic!("{} went wrong", 2));
        assert_eq!(out, Err("x crashed: 2 went wrong".to_owned()));
        assert_eq!(guarded("ok", || Ok(3)), Ok(3));
    }
}
