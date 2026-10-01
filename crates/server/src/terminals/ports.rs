//! What a terminal is listening on: the TCP ports held open for connections by
//! any process inside it.
//!
//! **What "inside it" is, is the platform's to say**, and each says it the way
//! its Sandbox holds a terminal together:
//!
//! - **Linux asks the terminal's pid namespace rather than its process tree.** A
//!   terminal's Sandbox is a `bwrap --unshare-all`, which puts everything the
//!   shell starts into a pid namespace of its own — and what a human starts in a
//!   terminal is free to `setsid`, double-fork and leave the shell's tree, which
//!   a server started with `&` and `disown` does. It cannot leave the namespace.
//!   So the namespace is found from the process the terminal was started as,
//!   and every process in it is read, wherever it hung itself in the tree.
//! - **macOS asks the tree and the process group.** A Seatbelt Sandbox makes no
//!   namespace, and the wrapper leads a process group of its own with a keeper
//!   beside it, so what is the terminal's is the wrapper, everything descended
//!   from it, and everything still in its group. See [`macos`].
//! - **Windows asks the Job.** A terminal there is a ConPTY under a Job Object
//!   of its own, and everything a process in a Job starts is in the Job too, so
//!   the Job's own list of processes is the tree with nothing to walk. See
//!   [`windows`].
//!
//! **Read the way `ss -p` reads it**, from outside, on Linux: each process's
//! open file descriptors listed under `/proc`, the sockets among them taken by
//! inode, and those inodes matched against the kernel's own tables of TCP
//! sockets — a row in `LISTEN` is a port. The same uid can read all of that
//! through the Sandbox's user namespace, which was measured from outside a
//! nested bwrap rather than assumed, and the suite measures it the same way. A
//! Mac has no `/proc`, so `lsof` is asked the same question of the pids found;
//! Windows keeps a table of TCP listeners with the pid that owns each, and the
//! Job's pids are matched against it.
//!
//! **Any address, either family**: a server bound to the loopback inside the
//! Sandbox is on the host's loopback, the network being shared, so what the
//! address was is no reason to leave a port out. UDP is: nothing is forwarded
//! over a datagram, so a datagram socket is not a port anybody could be offered.
//!
//! **And a reader that cannot tell reads nothing**, and says so once in the log
//! rather than on every turn: `lsof` missing, or a Job that will not list its
//! processes, is a terminal offering no ports — never a terminal that refuses
//! an attach, or one that reads as busy. Offering ports is a convenience on top
//! of a terminal, and a terminal is still a terminal without it.

use std::collections::BTreeSet;

use crate::terminal::Child;

/// What a terminal's ports are read from, as its register entry holds it: the
/// process the shell was started as on the platforms that read a tree from one,
/// and the Job on the platform whose terminal is one.
///
/// Taken off the [`Child`] when the terminal opens — see [`Tree::of`] — and
/// read on each turn with [`Tree::listening`].
#[derive(Clone, Debug)]
pub(crate) struct Tree(
    #[cfg(unix)] u32,
    #[cfg(windows)] std::sync::Weak<crate::sandbox::outliving::job::Job>,
);

#[cfg(unix)]
impl Tree {
    /// What `child` is read by: the process it was started as — `None` where
    /// it has already been reaped, which is a terminal that offers nothing.
    pub(crate) fn of(child: &Child) -> Option<Tree> {
        child.id().map(Tree)
    }

    /// The ports something in the terminal is listening on now, lowest first —
    /// see [`listening`].
    pub(crate) fn listening(&self) -> BTreeSet<u16> {
        listening(self.0)
    }
}

#[cfg(windows)]
impl Tree {
    /// What `child` is read by: the Job holding it, as a word that does not
    /// keep the Job — see [`Child::job`].
    pub(crate) fn of(child: &Child) -> Option<Tree> {
        Some(Tree(child.job()))
    }

    /// The ports something in the terminal is listening on now, lowest first.
    ///
    /// A Job whose `Child` has gone is a terminal that has gone, and listens on
    /// nothing: the Job is held only for as long as this read takes, so that a
    /// read is never what keeps one alive.
    pub(crate) fn listening(&self) -> BTreeSet<u16> {
        self.0
            .upgrade()
            .map(|job| windows::listening(&job))
            .unwrap_or_default()
    }
}

/// The ports something inside the terminal started as `leader` is listening on,
/// lowest first.
///
/// `leader` is the process the terminal's shell was started as — the Sandbox
/// wrapper, whose pid is the only one the server holds. A leader that has gone,
/// or a `/proc` that will not say, is a terminal listening on nothing: what this
/// is for is offering ports, and a port that cannot be read is one nobody is
/// offered.
#[cfg(target_os = "linux")]
pub fn listening(leader: u32) -> BTreeSet<u16> {
    linux::listening(leader)
}

/// And the same on macOS, off the leader's tree and process group by `ps` and
/// `lsof` — see [`macos`].
#[cfg(target_os = "macos")]
pub fn listening(leader: u32) -> BTreeSet<u16> {
    macos::listening(leader)
}

/// And nothing on a Unix that is neither: no Sandbox Verkstead builds runs
/// there, so there is no terminal to read.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
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

/// The reading on a Mac: the leader's processes found with `ps`, and their
/// listening sockets with `lsof`.
///
/// **The leader, its descendants, and its process group.** A Seatbelt Sandbox
/// is a `sandbox-exec` over the shell, which leads a process group of its own —
/// see [`crate::terminal`] — so a server the shell started is below it, and one
/// that double-forked out of the tree without a `setsid` is still in the group.
/// One that did both has left the terminal as far as this platform can say, and
/// is not read; the Mac has no namespace to hold it in.
///
/// **`lsof` filtered on its own side**, to TCP listeners held by those pids and
/// nothing else (`-a` makes the filters all apply rather than any), and asked
/// for its terse field output, whose `n` lines are each one socket's address and
/// port. It exits non-zero when it found nothing, which is a terminal listening
/// on nothing rather than a reader that could not tell — only a tool that would
/// not start at all is that.
///
/// Compiled into the tests on every platform, so that what is parsed out of the
/// two tools' output is proven where the suite runs most.
#[cfg(any(target_os = "macos", test))]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod macos {
    use std::collections::{BTreeSet, HashMap};
    use std::process::{Command, Stdio};
    use std::sync::Once;

    /// Where the two tools are on every Mac: a server started by launchd has a
    /// `PATH` of launchd's choosing, so they are named rather than looked for.
    const PS: &str = "/bin/ps";
    const LSOF: &str = "/usr/sbin/lsof";

    /// Whether a tool that would not start has been said already — once a
    /// server, rather than once a turn.
    static SAID: Once = Once::new();

    /// The reading itself — see [`super::listening`].
    pub(super) fn listening(leader: u32) -> BTreeSet<u16> {
        let Some(table) = ran(PS, &["-axo", "pid=,ppid=,pgid="]) else {
            return BTreeSet::new();
        };

        let inside = inside(leader, &processes(&table));

        if inside.is_empty() {
            return BTreeSet::new();
        }

        let pids = inside
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");

        ran(
            LSOF,
            &["-nP", "-a", "-iTCP", "-sTCP:LISTEN", "-p", &pids, "-Fn"],
        )
        .map(|listed| ports(&listed))
        .unwrap_or_default()
    }

    /// What `program` printed, whatever it exited with — or nothing, said once,
    /// where it would not start.
    fn ran(program: &str, args: &[&str]) -> Option<String> {
        match Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
        {
            Ok(output) => Some(String::from_utf8_lossy(&output.stdout).into_owned()),
            Err(error) => {
                SAID.call_once(|| {
                    tracing::warn!(
                        program,
                        error = %error,
                        "a terminal's ports cannot be read on this Mac, so none will be forwarded"
                    );
                });
                None
            }
        }
    }

    /// Each process `ps -o pid=,ppid=,pgid=` listed, as its pid, its parent and
    /// its process group.
    pub(super) fn processes(table: &str) -> Vec<(u32, u32, u32)> {
        table
            .lines()
            .filter_map(|row| {
                let mut fields = row.split_whitespace().map(str::parse);

                match (fields.next(), fields.next(), fields.next()) {
                    (Some(Ok(pid)), Some(Ok(parent)), Some(Ok(group))) => {
                        Some((pid, parent, group))
                    }
                    _ => None,
                }
            })
            .collect()
    }

    /// Every process that is the terminal's: `leader` itself, what descends from
    /// it, and what is in its process group — or nothing, where `leader` is not
    /// running at all.
    pub(super) fn inside(leader: u32, processes: &[(u32, u32, u32)]) -> BTreeSet<u32> {
        if !processes.iter().any(|&(pid, _, _)| pid == leader) {
            return BTreeSet::new();
        }

        let parents: HashMap<u32, u32> = processes
            .iter()
            .map(|&(pid, parent, _)| (pid, parent))
            .collect();

        let descended = |mut pid: u32| {
            while let Some(&above) = parents.get(&pid) {
                if above == leader {
                    return true;
                }
                if above <= 1 || above == pid {
                    return false;
                }
                pid = above;
            }
            false
        };

        processes
            .iter()
            .filter(|&&(pid, _, group)| pid == leader || group == leader || descended(pid))
            .map(|&(pid, _, _)| pid)
            .collect()
    }

    /// The ports in `lsof -F n` output: each `n` line is a socket's local
    /// address, which ends in its port after a colon — `*:8080`,
    /// `127.0.0.1:3000`, `[::1]:5173`.
    pub(super) fn ports(listed: &str) -> BTreeSet<u16> {
        listed
            .lines()
            .filter_map(|line| line.strip_prefix('n'))
            .filter_map(|address| address.rsplit_once(':')?.1.parse().ok())
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The leader, a child and a grandchild of it, something that left the
        /// tree but not the group, and two processes that are nobody's.
        #[test]
        fn the_tree_and_the_group_are_the_terminals_and_nothing_else_is() {
            let table = "    1     0     1
  500     1   500
  501   500   501
  502   501   501
  503     1   500
  504     1   504
  505   504   504
";

            let processes = processes(table);

            assert_eq!(processes.len(), 7);
            assert_eq!(
                inside(500, &processes),
                BTreeSet::from([500, 501, 502, 503])
            );
            assert!(inside(600, &processes).is_empty());
        }

        /// `lsof`'s terse output as it writes it: a process, its descriptor,
        /// and each socket's name — one wildcard, one IPv4, one IPv6.
        #[test]
        fn each_listening_name_is_a_port() {
            let listed = "p501\nf5\nn*:8080\np502\nf7\nn127.0.0.1:3000\nf8\nn[::1]:5173\n";

            assert_eq!(ports(listed), BTreeSet::from([3000, 5173, 8080]));
        }
    }
}

/// The reading on Windows: the Job's processes, matched against the table of
/// TCP listeners the IP Helper keeps with the pid that owns each.
///
/// Both families, IPv4 and IPv6, each its own table. A Job that will not list
/// its processes, or a table that will not be read, is a reader that cannot
/// tell: it reads nothing, and says so once.
#[cfg(windows)]
mod windows {
    use std::collections::{BTreeSet, HashSet};
    use std::ffi::c_void;
    use std::io;
    use std::sync::Once;

    use windows_sys::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, NO_ERROR};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCP6TABLE_OWNER_PID, MIB_TCPROW_OWNER_PID,
        MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
    };
    use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};

    use crate::sandbox::outliving::job::Job;

    /// Whether a reader that could not tell has been said already — once a
    /// server, rather than once a turn.
    static SAID: Once = Once::new();

    /// The ports something in `job` is listening on, lowest first.
    pub(super) fn listening(job: &Job) -> BTreeSet<u16> {
        match read(job) {
            Ok(ports) => ports,
            Err(error) => {
                SAID.call_once(|| {
                    tracing::warn!(
                        error = %error,
                        "a terminal's ports cannot be read on this machine, so none will be \
                         forwarded"
                    );
                });
                BTreeSet::new()
            }
        }
    }

    /// The reading, or why there is none.
    fn read(job: &Job) -> io::Result<BTreeSet<u16>> {
        let inside: HashSet<u32> = job.processes()?.into_iter().collect();

        if inside.is_empty() {
            return Ok(BTreeSet::new());
        }

        let mut ports = BTreeSet::new();

        let four = table(AF_INET)?;
        ports.extend(
            rows::<MIB_TCPROW_OWNER_PID>(
                &four,
                std::mem::offset_of!(MIB_TCPTABLE_OWNER_PID, table),
            )
            .filter(|row| inside.contains(&row.dwOwningPid))
            .map(|row| port(row.dwLocalPort)),
        );

        let six = table(AF_INET6)?;
        ports.extend(
            rows::<MIB_TCP6ROW_OWNER_PID>(
                &six,
                std::mem::offset_of!(MIB_TCP6TABLE_OWNER_PID, table),
            )
            .filter(|row| inside.contains(&row.dwOwningPid))
            .map(|row| port(row.dwLocalPort)),
        );

        Ok(ports)
    }

    /// One family's table of listeners with their owners, as the bytes the IP
    /// Helper wrote — in words, so that it is aligned for the rows in it.
    fn table(family: u16) -> io::Result<Vec<u64>> {
        let mut size = 0u32;
        let mut buffer: Vec<u64> = Vec::new();

        loop {
            // Safety: `buffer` is at least `size` bytes, and a null one with a
            // size of nothing is how the size is asked for.
            let answered = unsafe {
                GetExtendedTcpTable(
                    if buffer.is_empty() {
                        std::ptr::null_mut()
                    } else {
                        buffer.as_mut_ptr().cast::<c_void>()
                    },
                    &mut size,
                    0,
                    u32::from(family),
                    TCP_TABLE_OWNER_PID_LISTENER,
                    0,
                )
            };

            match answered {
                NO_ERROR => return Ok(buffer),
                // Asked for the size, or a table that grew between the asking
                // and the reading: room for what it says now, and again.
                ERROR_INSUFFICIENT_BUFFER => {
                    buffer = vec![0u64; (size as usize).div_ceil(size_of::<u64>()) + 1];
                    size = u32::try_from(buffer.len() * size_of::<u64>()).unwrap_or(u32::MAX);
                }
                code => return Err(io::Error::from_raw_os_error(code as i32)),
            }
        }
    }

    /// The rows of a table read by [`table`]: a count, then the rows from
    /// `first` bytes in.
    fn rows<Row: Copy>(table: &[u64], first: usize) -> impl Iterator<Item = Row> + '_ {
        let bytes = size_of_val(table);

        // Safety: a table always starts with its count, and every row read is
        // inside the buffer — the count is checked against its length.
        let count = if bytes >= size_of::<u32>() {
            unsafe { table.as_ptr().cast::<u32>().read() as usize }
        } else {
            0
        };
        let fits = bytes.saturating_sub(first) / size_of::<Row>();

        (0..count.min(fits)).map(move |at| unsafe {
            table
                .as_ptr()
                .cast::<u8>()
                .add(first + at * size_of::<Row>())
                .cast::<Row>()
                .read_unaligned()
        })
    }

    /// A port as the table writes one: the low two bytes, in network order.
    fn port(written: u32) -> u16 {
        u16::from_be((written & 0xFFFF) as u16)
    }

    #[cfg(test)]
    mod tests {
        use std::net::{Ipv4Addr, TcpListener};
        use std::os::windows::io::AsRawHandle;
        use std::time::{Duration, Instant};

        use super::*;
        use crate::terminals::ports::tests::{PATIENCE, free_port, listener};

        /// A listener in a Job is in the Job's reading, one beside it that is
        /// not in the Job is not, and the port leaves the reading once the
        /// process holding it has ended.
        #[test]
        fn a_listener_in_the_job_is_read_until_it_ends() {
            let port = free_port();
            let beside = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
            let elsewhere = beside.local_addr().unwrap().port();

            let job = Job::killing_everything_in_it().unwrap();
            let mut child = listener(port).spawn().unwrap();
            job.take(child.as_raw_handle()).unwrap();

            let deadline = Instant::now() + PATIENCE;

            loop {
                let read = listening(&job);
                assert!(
                    !read.contains(&elsewhere),
                    "{read:?} holds a port outside the Job"
                );

                if read.contains(&port) {
                    break;
                }

                assert!(
                    Instant::now() < deadline,
                    "port {port} in the Job was never read: {read:?}"
                );
                std::thread::sleep(Duration::from_millis(100));
            }

            child.kill().unwrap();
            child.wait().unwrap();

            let ended = Instant::now();

            while listening(&job).contains(&port) {
                assert!(
                    ended.elapsed() < Duration::from_secs(2),
                    "port {port} was still read two seconds after its process ended"
                );
                std::thread::sleep(Duration::from_millis(100));
            }
        }

        /// The one number the table is written in that is not what it looks
        /// like: 8080 in network order, in the low half of a word.
        #[test]
        fn a_port_is_read_out_of_network_order() {
            assert_eq!(port(u32::from(8080u16.to_be())), 8080);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::net::{Ipv4Addr, TcpListener};
    use std::process::Command;
    use std::time::Duration;

    /// How long a listener started for a test is waited for — it is a fresh
    /// test binary starting up — before the reading is said never to have
    /// seen it.
    pub(crate) const PATIENCE: Duration = Duration::from_secs(30);

    /// What tells [`a_listener_to_be_read`] which port to listen on, and that it
    /// is being run as one rather than by the suite.
    const LISTENER: &str = "VERKSTEAD_PORTS_TEST_LISTENER";

    /// A port nothing is on, a moment ago.
    pub(crate) fn free_port() -> u16 {
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    /// This test binary, asked to run [`a_listener_to_be_read`] on `port` — a
    /// listener of a known port in a process of its own, on every platform,
    /// with nothing installed to be one.
    pub(crate) fn listener(port: u16) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());

        command
            .args([
                "--exact",
                "terminals::ports::tests::a_listener_to_be_read",
                "--ignored",
                "--test-threads=1",
            ])
            .env(LISTENER, port.to_string());

        command
    }

    /// Not a test of anything: the listener [`listener`] starts, which listens
    /// on the port it was given until it is killed. Run by the suite, with no
    /// port given, it returns at once.
    #[test]
    #[ignore = "a listener the reading tests start in a process of its own"]
    fn a_listener_to_be_read() {
        let Ok(port) = std::env::var(LISTENER) else {
            return;
        };

        let _listening = TcpListener::bind((Ipv4Addr::LOCALHOST, port.parse::<u16>().unwrap()))
            .expect("the port should still be free");

        std::thread::sleep(Duration::from_secs(120));
    }

    /// The tree read from its leader, on the two platforms that read one from
    /// a pid: a listener the leader's shell started in the background is read,
    /// one beside it in this process is not, and the port leaves the reading
    /// within two seconds of the listener ending while the shell goes on.
    #[cfg(unix)]
    #[test]
    fn a_listener_below_the_leader_is_read_until_it_ends() {
        use std::io::{BufRead, BufReader};
        use std::os::unix::process::CommandExt;
        use std::process::Stdio;
        use std::time::Instant;

        let port = free_port();
        let beside = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let elsewhere = beside.local_addr().unwrap().port();

        let started = listener(port);
        let mut shell = Command::new("/bin/sh")
            .arg("-c")
            .arg("\"$0\" \"$@\" >/dev/null 2>&1 & echo $!; wait; exec sleep 60")
            .arg(started.get_program())
            .args(started.get_args())
            .env(LISTENER, port.to_string())
            .stdout(Stdio::piped())
            .process_group(0)
            .spawn()
            .unwrap();

        let mut said = String::new();
        BufReader::new(shell.stdout.take().unwrap())
            .read_line(&mut said)
            .unwrap();
        let background: i32 = said.trim().parse().unwrap();

        let leader = shell.id();
        let deadline = Instant::now() + PATIENCE;

        loop {
            let read = super::listening(leader);
            assert!(
                !read.contains(&elsewhere),
                "{read:?} holds a port outside the tree"
            );

            if read.contains(&port) {
                break;
            }

            assert!(
                Instant::now() < deadline,
                "port {port} below the leader was never read: {read:?}"
            );
            std::thread::sleep(Duration::from_millis(100));
        }

        rustix::process::kill_process(
            rustix::process::Pid::from_raw(background).unwrap(),
            rustix::process::Signal::KILL,
        )
        .unwrap();

        let ended = Instant::now();

        while super::listening(leader).contains(&port) {
            assert!(
                ended.elapsed() < Duration::from_secs(2),
                "port {port} was still read two seconds after its listener ended"
            );
            std::thread::sleep(Duration::from_millis(100));
        }

        assert!(
            shell.try_wait().unwrap().is_none(),
            "the leader should have outlived its listener, or this read nothing for a reason \
             other than the port closing"
        );

        let _ = rustix::process::kill_process_group(
            rustix::process::Pid::from_raw(leader.cast_signed()).unwrap(),
            rustix::process::Signal::KILL,
        );
        let _ = shell.wait();
    }
}
