//! What a terminal is listening on: the TCP ports held open for connections by
//! any process inside it.
//!
//! **Asked of the terminal's pid namespace rather than of its process tree.** A
//! terminal's Sandbox is a `bwrap --unshare-all`, which puts everything the
//! shell starts into a pid namespace of its own — and what a human starts in a
//! terminal is free to `setsid`, double-fork and leave the shell's tree, which a
//! server started with `&` and `disown` does. It cannot leave the namespace. So
//! the namespace is found from the process the terminal was started as, and
//! every process in it is read, wherever it hung itself in the tree.
//!
//! **Read the way `ss -p` reads it**, from outside: each process's open file
//! descriptors listed under `/proc`, the sockets among them taken by inode, and
//! those inodes matched against the kernel's own tables of TCP sockets — a row in
//! `LISTEN` is a port. The same uid can read all of that through the Sandbox's
//! user namespace, which was measured from outside a nested bwrap rather than
//! assumed, and the suite measures it the same way.
//!
//! **Any address, either family**: a server bound to the loopback inside the
//! Sandbox is on the host's loopback, the network being shared, so what the
//! address was is no reason to leave a port out. UDP is: nothing is forwarded
//! over a datagram, so a datagram socket is not a port anybody could be offered.
//!
//! **Nothing at all on the other platforms yet** — see [`listening`], which says
//! so on each.

use std::collections::BTreeSet;

/// The ports something inside the terminal started as `leader` is listening on,
/// lowest first.
///
/// `leader` is the process the terminal's shell was started as — the Sandbox
/// wrapper on every platform, whose pid is the only one the server holds. A
/// leader that has gone, or a `/proc` that will not say, is a terminal listening
/// on nothing: what this is for is offering ports, and a port that cannot be
/// read is one nobody is offered.
#[cfg(target_os = "linux")]
pub fn listening(leader: u32) -> BTreeSet<u16> {
    linux::listening(leader)
}

/// And nothing, on macOS: a Seatbelt Sandbox makes no namespace to find the
/// terminal's processes by, and its own reading — the process group and the
/// session, over `libproc` — is a later piece of work than this one.
#[cfg(target_os = "macos")]
pub fn listening(_leader: u32) -> BTreeSet<u16> {
    BTreeSet::new()
}

/// And nothing, on Windows: what a terminal starts there is held by a Job
/// Object, and the reading is that Job's processes against the TCP table the
/// IP Helper keeps — a later piece of work than this one.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn listening(_leader: u32) -> BTreeSet<u16> {
    BTreeSet::new()
}

#[cfg(target_os = "linux")]
mod linux {
    use std::collections::{BTreeSet, HashMap, HashSet};
    use std::fs;
    use std::path::Path;

    /// What the kernel's tables call a socket that is listening: `TCP_LISTEN`,
    /// in the hexadecimal they are written in.
    const LISTEN: &str = "0A";

    /// The reading itself — see [`super::listening`].
    pub(super) fn listening(leader: u32) -> BTreeSet<u16> {
        let inside = inside(leader);

        let Some(&reading) = inside.iter().next() else {
            return BTreeSet::new();
        };

        let held: HashSet<u64> = inside.iter().flat_map(|&pid| sockets(pid)).collect();

        if held.is_empty() {
            return BTreeSet::new();
        }

        // The tables of the network the terminal is in, read through one of its
        // own processes: the host's while the Sandbox shares it, and its own if
        // a Sandbox ever does not.
        ["tcp", "tcp6"]
            .into_iter()
            .filter_map(|table| fs::read_to_string(format!("/proc/{reading}/net/{table}")).ok())
            .flat_map(|table| listeners(&table))
            .filter(|(inode, _)| held.contains(inode))
            .map(|(_, port)| port)
            .collect()
    }

    /// Every process that is the terminal's: those in the pid namespace below
    /// `leader`, and `leader`'s descendants besides.
    ///
    /// The namespace is the one any of `leader`'s descendants is in that
    /// `leader` itself is not — the wrapper starts outside it and forks its way
    /// in. The descendants are counted too, so that a Sandbox that made no
    /// namespace is a tree read rather than nothing read.
    fn inside(leader: u32) -> BTreeSet<u32> {
        let Ok(listed) = fs::read_dir("/proc") else {
            return BTreeSet::new();
        };

        let pids: Vec<u32> = listed
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().to_str()?.parse().ok())
            .collect();

        let parents: HashMap<u32, u32> = pids
            .iter()
            .filter_map(|&pid| Some((pid, parent(pid)?)))
            .collect();

        let descended = |mut pid: u32| {
            while let Some(&above) = parents.get(&pid) {
                if above == leader {
                    return true;
                }
                if above <= 1 {
                    return false;
                }
                pid = above;
            }
            false
        };

        let mut found: BTreeSet<u32> = pids.iter().copied().filter(|&pid| descended(pid)).collect();

        let outside = namespace(leader);
        let theirs = found
            .iter()
            .filter_map(|&pid| namespace(pid))
            .find(|namespace| Some(namespace) != outside.as_ref());

        if let Some(theirs) = theirs {
            found.extend(
                pids.iter()
                    .copied()
                    .filter(|&pid| namespace(pid).as_ref() == Some(&theirs)),
            );
        }

        found
    }

    /// The parent of `pid`, off its `stat` — after the name, which is in
    /// brackets and may itself hold a bracket or a space.
    fn parent(pid: u32) -> Option<u32> {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let (_, after) = stat.rsplit_once(')')?;

        // State, then the parent.
        after.split_whitespace().nth(1)?.parse().ok()
    }

    /// Which pid namespace `pid` is in, as the link the kernel names it by.
    fn namespace(pid: u32) -> Option<std::path::PathBuf> {
        fs::read_link(format!("/proc/{pid}/ns/pid")).ok()
    }

    /// The inodes of the sockets `pid` holds open.
    fn sockets(pid: u32) -> Vec<u64> {
        let fds = Path::new("/proc").join(pid.to_string()).join("fd");

        let Ok(listed) = fs::read_dir(fds) else {
            return Vec::new();
        };

        listed
            .filter_map(Result::ok)
            .filter_map(|entry| fs::read_link(entry.path()).ok())
            .filter_map(|target| {
                target
                    .to_str()?
                    .strip_prefix("socket:[")?
                    .strip_suffix(']')?
                    .parse()
                    .ok()
            })
            .collect()
    }

    /// The listening rows of one of the kernel's TCP tables, as each socket's
    /// inode and the port it is on.
    ///
    /// A row is `sl local rem st tx:rx tr:when retrnsmt uid timeout inode …`,
    /// and the local address ends in the port, in hexadecimal, after a colon.
    fn listeners(table: &str) -> Vec<(u64, u16)> {
        table
            .lines()
            .skip(1)
            .filter_map(|row| {
                let fields: Vec<&str> = row.split_whitespace().collect();

                if fields.get(3) != Some(&LISTEN) {
                    return None;
                }

                let (_, port) = fields.get(1)?.rsplit_once(':')?;
                let port = u16::from_str_radix(port, 16).ok()?;
                let inode = fields.get(9)?.parse().ok()?;

                Some((inode, port))
            })
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// A table as the kernel writes one: a listener on the loopback, one on
        /// every address, and a connection that is not listening at all.
        #[test]
        fn only_the_listening_rows_of_a_table_are_ports() {
            let table = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 41234 1 0000000000000000 100 0 0 10 0
   1: 00000000:0BB8 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 41235 1 0000000000000000 100 0 0 10 0
   2: 0100007F:A1B2 0100007F:1F90 01 00000000:00000000 00:00000000 00000000  1000        0 41236 1 0000000000000000 20 4 30 10 -1
";

            assert_eq!(listeners(table), vec![(41234, 8080), (41235, 3000)]);
        }
    }
}
