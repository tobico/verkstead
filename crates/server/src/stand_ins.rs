//! The programs the tests put where a real one would be, written so that they
//! will run.
//!
//! One module for what looks like two lines of every test module's own setup,
//! because those two lines are wrong in a way that only shows up under load:
//! see [`program`].

use std::path::Path;
use std::sync::{Arc, Mutex};

/// A program at `path` holding `contents`, executable where this platform has
/// such a thing.
///
/// The bytes go in through a child rather than from here, and that is what this
/// is for. A descriptor this process opens on the file is copied into every
/// fork a sibling thread makes for as long as it is open, and it stays in that
/// fork until the fork's own exec — while Linux refuses to run a file that
/// anybody holds open for writing. A test that wrote its stand-in from this
/// thread would meet `ETXTBSY` whenever another test happened to spawn
/// something during the write, which reaches it as the machine saying the
/// stand-in is not a program: a failure with nothing wrong in it, and one that
/// only turns up on a machine running the suite in parallel.
///
/// The writing descriptor belongs to the child doing the writing, where no fork
/// of ours can pick it up. Nothing else in the sequence opens the file — a mode
/// is set on the path rather than through a handle — so what comes back is a
/// file this process has never held open, and running it is not a race.
///
/// Windows writes it from here: there is no such refusal to work around, and no
/// mode bit that makes a file a program.
pub(crate) fn program(path: &Path, contents: &str) {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        use std::process::{Command, Stdio};

        let mut writing = Command::new("/bin/sh")
            .arg("-c")
            .arg(r#"cat > "$1""#)
            .arg("sh")
            .arg(path)
            .stdin(Stdio::piped())
            .spawn()
            .expect("a shell to write the stand-in with");

        writing
            .stdin
            .take()
            .expect("the shell was given a pipe")
            .write_all(contents.as_bytes())
            .expect("the shell to take the stand-in");

        let wrote = writing.wait().expect("the shell to finish");
        assert!(
            wrote.success(),
            "the stand-in at {} went unwritten: {wrote}",
            path.display()
        );

        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(not(unix))]
    std::fs::write(path, contents).unwrap();
}

/// A remote over http that asks for a login and writes down every one it is
/// offered, as the `user:password` git sent — and then turns it away, since
/// whose credential arrived is the whole of what a test of it asks.
///
/// Hands back the url to point a remote at, and the logins as they arrive.
pub(crate) fn remote_recording_logins() -> (String, Arc<Mutex<Vec<String>>>) {
    use base64::Engine;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let offered = Arc::new(Mutex::new(Vec::new()));

    std::thread::spawn({
        let offered = offered.clone();

        move || {
            for mut stream in listener.incoming().flatten() {
                let mut login = None;

                for line in BufReader::new(&stream).lines() {
                    let Ok(line) = line else { break };

                    if line.is_empty() {
                        break;
                    }

                    let Some((name, value)) = line.split_once(": ") else {
                        continue;
                    };

                    if let ("authorization", Some(basic)) = (
                        name.to_ascii_lowercase().as_str(),
                        value.strip_prefix("Basic "),
                    ) {
                        let decoded = base64::engine::general_purpose::STANDARD
                            .decode(basic.trim())
                            .unwrap();
                        login = Some(String::from_utf8(decoded).unwrap());
                    }
                }

                // Asked for where nothing was offered, which is what has git go
                // to its credential helper; refused where something was.
                let answer = match login {
                    Some(login) => {
                        offered.lock().unwrap().push(login);
                        "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    }
                    None => {
                        "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"repo\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    }
                };

                let _ = stream.write_all(answer.as_bytes());
            }
        }
    });

    (format!("http://127.0.0.1:{port}/repo.git"), offered)
}
