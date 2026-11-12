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
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rune::{Tx, Waker};

use crate::app::State;

mod remote;

pub use remote::{Remote, load, load_then};

type Landing = Box<dyn FnOnce(&mut State, &mut Tx) + Send>;

/// A job, ready to go: called with `None` it does its work, with `Some(error)` it skips the
/// work (it couldn't be started) and lands that error. Either way it lands exactly once.
type Task = Box<dyn FnOnce(Option<String>) + Send>;

/// Where finished work goes: the UI thread's inbox, and the waker that gets it to look.
#[derive(Clone)]
struct Outbox {
    send: Sender<Landing>,
    waker: Option<Waker>,
}

impl Outbox {
    /// Hands `landing` to the UI thread and wakes it; false once the app is gone.
    fn deliver(&self, landing: Landing) -> bool {
        let delivered = self.send.send(landing).is_ok();
        if delivered && let Some(w) = &self.waker {
            w();
        }
        delivered
    }
}

pub struct Jobs {
    outbox: Outbox,
    inbox: Receiver<Landing>,
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
            outbox: Outbox { send, waker: None },
            inbox,
            live: false,
            writer: None,
            skipped: Vec::new(),
        }
    }

    pub fn set_waker(&mut self, waker: Waker) {
        self.outbox.waker = Some(waker);
    }

    pub fn go_live(&mut self) {
        self.live = true;
    }

    /// True (and the job recorded) when jobs don't run yet.
    fn skip(&mut self, name: &'static str) -> bool {
        if !self.live {
            self.skipped.push(name);
        }
        !self.live
    }

    /// Reads: runs `work` on its own worker thread, then `land` with its answer on the UI
    /// thread. Reads may run alongside each other and alongside a write.
    pub fn run<T: Send + 'static>(
        &mut self,
        name: &'static str,
        work: impl FnOnce() -> Result<T, String> + Send + 'static,
        land: impl FnOnce(&mut State, Result<T, String>, &mut Tx) + Send + 'static,
    ) {
        if self.skip(name) {
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
        if self.skip(name) {
            return;
        }
        let task = self.task(name, work, land);
        let (task, why) = match self.writer() {
            Ok(writer) => match writer.send(task) {
                Ok(()) => return,
                Err(refused) => {
                    // The writer is gone (it never panics, so this is shutdown); don't queue again.
                    self.writer = None;
                    (refused.0, "the writer stopped".to_owned())
                }
            },
            Err(e) => (task, e.to_string()),
        };
        task(Some(format!("Couldn't start {name}: {why}")));
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
        if self.skip(name) {
            return;
        }
        let outbox = self.outbox.clone();
        let spawned = std::thread::Builder::new()
            .name(format!("stream-{name}"))
            .spawn(move || {
                let mut emit = |value: T| {
                    let land = land.clone();
                    outbox.deliver(Box::new(move |s: &mut State, tx: &mut Tx| {
                        land(s, value, tx)
                    }))
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
        let outbox = self.outbox.clone();
        Box::new(move |refused: Option<String>| {
            let out = match refused {
                Some(e) => Err(e),
                None => guarded(name, work),
            };
            outbox.deliver(Box::new(move |s: &mut State, tx: &mut Tx| land(s, out, tx)));
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

fn panic_message(p: &(dyn Any + Send)) -> &str {
    p.downcast_ref::<&str>()
        .copied()
        .or_else(|| p.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown panic")
}

#[cfg(test)]
mod tests {
    use super::{Settle, guarded};

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
