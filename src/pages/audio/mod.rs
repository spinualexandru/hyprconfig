//! The audio page: PipeWire devices and application streams, with volume changes debounced
//! and queued so only the newest volume for each node is sent.

mod view;

pub use view::view;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rune::Tx;

use crate::app::{self, State};
use crate::backend::audio::{self as backend, AudioState as Mixer};
use crate::jobs::{self, Remote};

/// Seconds a volume waits for the slider to stop before it's sent to PipeWire.
const DEBOUNCE: f64 = 0.1;
pub const MAX_VOLUME: f32 = 1.5;

#[derive(Default)]
pub struct AudioState {
    pub mixer: Remote<Mixer>,
    /// Volumes moved on screen but not yet sent: node, volume, when to send.
    pub pending: Vec<(u32, f32, f64)>,
    /// The newest volume sent for each node, by number. Volumes queue on the write lane in
    /// order; one that a newer volume for its node overtook in the queue is skipped.
    latest: Arc<Mutex<HashMap<u32, u64>>>,
    sent: u64,
}

/// Whether volume number `n` is still the newest for `node`.
fn newest(latest: &Mutex<HashMap<u32, u64>>, node: u32, n: u64) -> bool {
    latest
        .lock()
        .map_or(true, |l| l.get(&node).is_none_or(|&m| m == n))
}

pub fn load(s: &mut State) {
    jobs::load(
        s,
        "audio-state",
        |s| &mut s.audio.mixer,
        backend::get_audio_state,
    );
}

fn each_node(m: &mut Mixer, id: u32, mut f: impl FnMut(&mut f32, &mut bool)) {
    for d in m
        .sinks
        .iter_mut()
        .chain(m.sources.iter_mut())
        .filter(|d| d.id == id)
    {
        f(&mut d.volume, &mut d.muted);
    }
    for st in m.streams.iter_mut().filter(|st| st.id == id) {
        f(&mut st.volume, &mut st.muted);
    }
}

pub fn set_volume(s: &mut State, id: u32, volume: f32, tx: &mut Tx) {
    let volume = (volume * 100.0).round() / 100.0;
    if let Some(m) = s.audio.mixer.data.as_mut() {
        each_node(m, id, |v, _| *v = volume);
    }
    let due = tx.clock() + DEBOUNCE;
    s.audio.pending.retain(|(n, ..)| *n != id);
    s.audio.pending.push((id, volume, due));
    tx.wake_at(due);
}

/// Sends volumes whose slider has rested. Changes nothing on screen.
pub fn flush(s: &mut State, now: f64) -> bool {
    let (due, waiting): (Vec<_>, Vec<_>) =
        s.audio.pending.drain(..).partition(|(.., at)| *at <= now);
    s.audio.pending = waiting;
    for (id, volume, _) in due {
        s.audio.sent += 1;
        let n = s.audio.sent;
        if let Ok(mut l) = s.audio.latest.lock() {
            l.insert(id, n);
        }
        let latest = Arc::clone(&s.audio.latest);
        s.jobs.write(
            "set-volume",
            move || {
                if !newest(&latest, id, n) {
                    return Ok(());
                }
                backend::set_volume(id, volume)
            },
            |s, r, tx| {
                if let Err(e) = r {
                    app::fail(s, tx, "Failed to set volume", e);
                }
            },
        );
    }
    false
}

fn toggle_mute(s: &mut State, id: u32) {
    let mut muted = false;
    if let Some(m) = s.audio.mixer.data.as_mut() {
        each_node(m, id, |_, mu| {
            *mu = !*mu;
            muted = *mu;
        });
    }
    s.jobs.write(
        "set-mute",
        move || backend::set_mute(id, muted),
        |s, r, tx| {
            if let Err(e) = r {
                app::fail(s, tx, "Failed to toggle mute", e);
                load(s);
            }
        },
    );
}

fn set_default(s: &mut State, id: u32) {
    s.jobs.write(
        "set-default-device",
        move || backend::set_default_device(id),
        |s, r, tx| match r {
            Ok(()) => load(s),
            Err(e) => app::fail(s, tx, "Failed to set default device", e),
        },
    );
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::newest;

    #[test]
    fn only_the_newest_queued_volume_for_a_node_is_sent() {
        let latest = Mutex::new(HashMap::from([(42, 7), (51, 3)]));
        assert!(!newest(&latest, 42, 6));
        assert!(newest(&latest, 42, 7));
        assert!(newest(&latest, 51, 3));
        assert!(newest(&latest, 9, 1));
    }
}
