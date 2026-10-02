//! A terminal printing an address on this machine, which is a server saying it
//! is up — and a reason to read the terminal's ports now rather than on the
//! next turn of the reading.
//!
//! **A hint and nothing more** (Set 999, Q13). What is forwarded is what the
//! reading finds, and the reading is still the walk of what the terminal's tree
//! is listening on: a port printed that nothing is listening on is a port
//! nobody offered, whatever the output said about it. What a print buys is
//! *when* the read happens — a dev server says its address the moment it is up,
//! and two seconds is a long time to look at *cannot connect*.
//!
//! **What counts** is a URL naming this machine — `localhost`, `127.0.0.1`,
//! `0.0.0.0` or `[::1]` — with a port of its own, anywhere in what the screen
//! receives. Read past the colouring a terminal's output is full of: a dev
//! server's address is the place it is most likely to put its bold.
//!
//! **One read per burst.** A hint wakes the read only where it is waiting
//! between turns, so a server that prints its URL three times while the read it
//! started is walking is read once; and the turns carry on from there as they
//! would have — see [`Hurry`].
//!
//! **And only while the terminal is being read at all**, which is while a member
//! holds an attach on it: output nobody is reading the ports of is not looked
//! at.

use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Notify;
use tokio::sync::futures::Notified;

/// The hosts that are this machine, as an address in a URL names them.
const HERE: [&[u8]; 4] = [b"localhost", b"127.0.0.1", b"0.0.0.0", b"[::1]"];

/// How much of one read is kept for the next, so that an address the pty
/// handed over in two pieces is still one address.
///
/// Room for the longest URL prefix there is to find, with its colouring.
const CARRIED: usize = 256;

/// Whether a terminal is being read, and the word that hurries the read along —
/// shared between the register, the read and the relay following the output.
#[derive(Default)]
pub(crate) struct Hurry {
    reading: AtomicBool,
    now: Notify,
}

impl Hurry {
    /// Mark the read as running, saying whether it was not already — which is
    /// whether to start one. Called under the register's lock, as its stopping
    /// is.
    pub(crate) fn start(&self) -> bool {
        !self.reading.swap(true, Ordering::Relaxed)
    }

    /// And mark it stopped, which is the sniff stopping with it.
    pub(crate) fn stop(&self) {
        self.reading.store(false, Ordering::Relaxed);
    }

    /// Whether the read is running, which is whether the output is worth looking
    /// at.
    pub(crate) fn reading(&self) -> bool {
        self.reading.load(Ordering::Relaxed)
    }

    /// Wake the read where it is waiting for its next turn.
    ///
    /// A read that is walking already hears nothing: no word is kept for later,
    /// so the prints of one burst are one read between them.
    pub(crate) fn hurry(&self) {
        self.now.notify_waiters();
    }

    /// What the read waits on beside its turn — taken once a walk is over, so
    /// that only what is printed after it hurries the next one.
    pub(crate) fn hurried(&self) -> Notified<'_> {
        self.now.notified()
    }
}

/// The sniff over one terminal's output, holding the end of what it was last
/// handed.
#[derive(Default)]
pub(crate) struct Sniffing {
    carried: Vec<u8>,
}

impl Sniffing {
    /// Whether `bytes`, read on from what came before, print an address on this
    /// machine with a port.
    ///
    /// An address found is not found again: what was carried goes with it, so
    /// the same URL is never a second hint for coming round in the carry.
    pub(crate) fn names_here(&mut self, bytes: &[u8]) -> bool {
        self.carried.extend_from_slice(bytes);

        if names_here(&shown(&self.carried)) {
            self.carried.clear();
            return true;
        }

        let keep = self.carried.len().saturating_sub(CARRIED);
        self.carried.drain(..keep);

        false
    }
}

/// What of `bytes` reaches the screen as text: the escape sequences taken out.
///
/// A control sequence (`ESC [` … a final byte), an operating system command
/// (`ESC ]` … BEL or `ESC \`) and any other escape with the one byte after it.
/// One cut short at the end is dropped whole — it is carried, and read whole
/// with what follows it.
fn shown(bytes: &[u8]) -> Vec<u8> {
    let mut text = Vec::with_capacity(bytes.len());
    let mut at = 0;

    while at < bytes.len() {
        if bytes[at] != 0x1b {
            text.push(bytes[at]);
            at += 1;
            continue;
        }

        at = match bytes.get(at + 1) {
            Some(b'[') => bytes[at + 2..]
                .iter()
                .position(|&byte| (0x40..=0x7e).contains(&byte))
                .map_or(bytes.len(), |end| at + 2 + end + 1),
            Some(b']') => {
                let rest = &bytes[at + 2..];

                match rest.iter().enumerate().find(|&(i, &byte)| {
                    byte == 0x07 || (byte == 0x1b && rest.get(i + 1) == Some(&b'\\'))
                }) {
                    Some((end, 0x07)) => at + 2 + end + 1,
                    Some((end, _)) => at + 2 + end + 2,
                    None => bytes.len(),
                }
            }
            Some(_) => at + 2,
            None => bytes.len(),
        };
    }

    text
}

/// Whether `text` holds `://` followed by one of [`HERE`], a colon and a port
/// that is over — digits with something after them that is not one.
fn names_here(text: &[u8]) -> bool {
    text.windows(3)
        .enumerate()
        .filter(|(_, window)| *window == b"://")
        .any(|(at, _)| {
            let after = &text[at + 3..];

            HERE.iter().any(|host| {
                after.len() > host.len()
                    && after[..host.len()].eq_ignore_ascii_case(host)
                    && after[host.len()] == b':'
                    && ported(&after[host.len() + 1..])
            })
        })
}

/// Whether `text` opens with a port: one to five digits naming one, ended by
/// something that is not a digit.
fn ported(text: &[u8]) -> bool {
    let digits = text.iter().take_while(|byte| byte.is_ascii_digit()).count();

    (1..=5).contains(&digits)
        && digits < text.len()
        && std::str::from_utf8(&text[..digits])
            .ok()
            .and_then(|port| port.parse::<u16>().ok())
            .is_some_and(|port| port != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sniffed(chunks: &[&[u8]]) -> Vec<bool> {
        let mut sniffing = Sniffing::default();

        chunks
            .iter()
            .map(|chunk| sniffing.names_here(chunk))
            .collect()
    }

    /// Each of the names this machine goes by, as a server prints it.
    #[test]
    fn an_address_on_this_machine_with_a_port_is_a_hint() {
        for printed in [
            "Listening on http://localhost:3000\r\n",
            "  ➜  Local:   http://127.0.0.1:5173/\r\n",
            "Serving HTTP on 0.0.0.0 port 8000 (http://0.0.0.0:8000/) ...\r\n",
            "ready at https://[::1]:8443/app\r\n",
            "-> HTTP://LocalHost:4000 \r\n",
        ] {
            assert_eq!(sniffed(&[printed.as_bytes()]), [true], "{printed:?}");
        }
    }

    /// An address somewhere else, one without a port and something that is not
    /// a URL at all.
    #[test]
    fn anything_else_is_not() {
        for printed in [
            "http://example.com:3000/\r\n",
            "http://localhost/\r\n",
            "localhost:3000\r\n",
            "http://localhost:99999/\r\n",
            "http://localhost:0/\r\n",
            "http://localhost.example.com:3000/\r\n",
        ] {
            assert_eq!(sniffed(&[printed.as_bytes()]), [false], "{printed:?}");
        }
    }

    /// Vite bolds its port, and a terminal may wrap the whole address in a
    /// hyperlink.
    #[test]
    fn colouring_is_read_past() {
        assert_eq!(
            sniffed(&[b"\x1b[36mhttp://localhost:\x1b[1m5173\x1b[22m/\x1b[39m\r\n"]),
            [true],
        );
        assert_eq!(
            sniffed(&[b"\x1b]8;;http://localhost:3000\x1b\\http://localhost:3000\x1b]8;;\x07\r\n"]),
            [true],
        );
    }

    /// Handed over in pieces, it is still one address — and one hint, however
    /// it was split.
    #[test]
    fn an_address_split_between_reads_is_one_hint() {
        assert_eq!(
            sniffed(&[b"up at http://local", b"host:30", b"00/\r\n", b"$ "]),
            [false, false, true, false],
        );
        assert_eq!(
            sniffed(&[b"http://localhost:\x1b[1", b"m8080\x1b[22m\r\n"]),
            [false, true],
        );
    }

    /// A port cut off at the end of a read could be the front of a longer one,
    /// so it waits for what follows.
    #[test]
    fn a_port_at_the_very_end_waits_for_the_next_read() {
        assert_eq!(sniffed(&[b"http://localhost:30", b"\r\n"]), [false, true]);
    }

    /// A read the hint came too early for is not woken by it later.
    #[tokio::test]
    async fn a_hint_is_not_kept_for_a_read_that_was_not_waiting() {
        let hurry = Hurry::default();

        hurry.hurry();

        let waited =
            tokio::time::timeout(std::time::Duration::from_millis(50), hurry.hurried()).await;
        assert!(waited.is_err(), "a hint nobody was waiting on was kept");

        let hurried = hurry.hurried();
        hurry.hurry();
        tokio::time::timeout(std::time::Duration::from_millis(50), hurried)
            .await
            .expect("a read waiting is woken");
    }
}
