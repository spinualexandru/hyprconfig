//! `Remote<T>`: data loaded from the system on the UI side, with the attempt that keeps stale
//! answers out and the hold that marks a write in flight. `load`/`load_then` fill one from a job.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::app::State;

/// Loads the `Remote` that `pick` points at with `work`, on a read worker; the answer lands
/// there unless a newer load superseded it.
pub fn load<T: Send + 'static>(
    s: &mut State,
    name: &'static str,
    pick: fn(&mut State) -> &mut Remote<T>,
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) {
    load_then(s, name, pick, work, |_| {});
}

/// `load`, then `then` once the answer was taken.
pub fn load_then<T: Send + 'static>(
    s: &mut State,
    name: &'static str,
    pick: fn(&mut State) -> &mut Remote<T>,
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
    then: impl FnOnce(&mut State) + Send + 'static,
) {
    let attempt = pick(s).begin();
    s.jobs.run(name, work, move |s, r, _| {
        if pick(s).land(attempt, r) {
            then(s);
        }
    });
}

/// Attempt numbers are unique across every `Remote`, so an answer to a request made before a
/// `Remote` was replaced can't match one made after.
fn next_attempt() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hold {
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

    /// A write is changing what this data describes. Until `after_write(false)`, or the
    /// reload after `after_write(true)` lands, rows mustn't be acted on by their position.
    pub fn hold(&mut self) {
        self.hold = Hold::Writing;
    }

    /// The held write landed. If it succeeded, the data is stale until the next accepted
    /// answer (reload right after); if it failed, the data still describes the file.
    pub fn after_write(&mut self, ok: bool) {
        self.hold = if ok { Hold::Reloading } else { Hold::None };
    }

    /// Whether a write or its reload is outstanding.
    pub fn settling(&self) -> bool {
        self.hold != Hold::None
    }
}

#[cfg(test)]
mod tests {
    use super::Remote;

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
        r.after_write(true);
        let after = r.begin();
        assert!(r.settling());
        assert!(r.land(after, Ok(vec![1, 3])));
        assert!(!r.settling());

        r.hold();
        r.after_write(false);
        assert!(!r.settling());
    }
}
