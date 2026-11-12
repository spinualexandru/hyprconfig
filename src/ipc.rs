//! A Unix socket the running app listens on, so other programs can tell it something changed.
//! Noctalia's hooks run `hyprconfig notify <event>`, which writes the event's name to the
//! socket and exits; when no Hyprconfig is running there's nobody to tell, and it exits quietly.

use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// The colour palette changed.
    Colors,
    /// The light or dark mode changed.
    Mode,
    /// The wallpaper changed.
    Wallpaper,
}

impl Event {
    pub const ALL: [Event; 3] = [Event::Colors, Event::Mode, Event::Wallpaper];

    pub fn name(self) -> &'static str {
        match self {
            Event::Colors => "colors",
            Event::Mode => "mode",
            Event::Wallpaper => "wallpaper",
        }
    }

    pub fn parse(s: &str) -> Option<Event> {
        Event::ALL.into_iter().find(|e| e.name() == s.trim())
    }
}

pub fn socket_path() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|d| !d.is_empty())
        .map(|d| PathBuf::from(d).join("hyprconfig.sock"))
}

/// Tells a running Hyprconfig about `event`. Nobody listening isn't an error.
pub fn notify(event: &str) -> Result<(), String> {
    let event = Event::parse(event).ok_or_else(|| {
        let names: Vec<_> = Event::ALL.iter().map(|e| e.name()).collect();
        format!(
            "unknown event \"{event}\"; expected one of {}",
            names.join(", ")
        )
    })?;
    let Some(path) = socket_path() else {
        return Ok(());
    };
    match UnixStream::connect(&path) {
        Ok(mut stream) => {
            let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
            stream
                .write_all(format!("{}\n", event.name()).as_bytes())
                .map_err(|e| format!("couldn't reach Hyprconfig: {e}"))
        }
        Err(e) if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::ConnectionRefused) => Ok(()),
        Err(e) => Err(format!("couldn't reach Hyprconfig: {e}")),
    }
}

/// Starts listening. A socket left behind by a Hyprconfig that's gone is replaced; one that
/// still answers belongs to another running Hyprconfig, which keeps it.
pub fn bind() -> Result<UnixListener, String> {
    let path = socket_path().ok_or("XDG_RUNTIME_DIR isn't set")?;
    match UnixListener::bind(&path) {
        Ok(l) => Ok(l),
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            if UnixStream::connect(&path).is_ok() {
                return Err("another Hyprconfig is already listening".into());
            }
            std::fs::remove_file(&path)
                .map_err(|e| format!("couldn't replace {}: {e}", path.display()))?;
            UnixListener::bind(&path).map_err(|e| format!("couldn't listen: {e}"))
        }
        Err(e) => Err(format!("couldn't listen on {}: {e}", path.display())),
    }
}

/// Hands each event that arrives to `each` until it returns false.
pub fn serve(listener: UnixListener, mut each: impl FnMut(Event) -> bool) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        // A client that connects and says nothing mustn't hold up the next one.
        let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
        let mut line = String::new();
        if (&mut stream).take(64).read_to_string(&mut line).is_err() {
            continue;
        }
        if let Some(event) = Event::parse(&line)
            && !each(event)
        {
            return;
        }
    }
}

/// Like `serve`, but events that arrive within `window` of each other come as one batch, each
/// event once: Noctalia reports a new wallpaper and then the colours made from it, and one
/// recolouring covers both.
pub fn serve_batched(
    listener: UnixListener,
    window: Duration,
    mut each: impl FnMut(Vec<Event>) -> bool,
) {
    let (send, events) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("ipc-accept".into())
        .spawn(move || serve(listener, |e| send.send(e).is_ok()));
    if spawned.is_err() {
        return;
    }
    while let Ok(first) = events.recv() {
        let mut batch = vec![first];
        while let Ok(e) = events.recv_timeout(window) {
            if !batch.contains(&e) {
                batch.push(e);
            }
        }
        if !each(batch) {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_round_trip() {
        for e in Event::ALL {
            assert_eq!(Event::parse(e.name()), Some(e));
        }
        assert_eq!(Event::parse("colors\n"), Some(Event::Colors));
        assert_eq!(Event::parse("battery"), None);
        assert!(notify("battery").is_err());
    }

    #[test]
    fn a_notification_reaches_the_listener() {
        let path = std::env::temp_dir().join(format!("hyprconfig-ipc-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let client = path.clone();
        let sender = std::thread::spawn(move || {
            for e in ["mode", "nonsense", "colors"] {
                let mut s = UnixStream::connect(&client).unwrap();
                s.write_all(e.as_bytes()).unwrap();
            }
        });
        let mut got = Vec::new();
        serve(listener, |e| {
            got.push(e);
            got.len() < 2
        });
        sender.join().unwrap();
        assert_eq!(got, vec![Event::Mode, Event::Colors]);
        std::fs::remove_file(&path).unwrap();
    }
}
