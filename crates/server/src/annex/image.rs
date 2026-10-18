//! Downloading the image an Annex's box is made from, and checking it.
//!
//! **Checked against the release's own checksum**, which is a file beside the
//! image as `sha256sum` prints one: what is downloaded is half a gigabyte from
//! somebody else's release, and a box imported from a file cut short would be
//! a broken box that said nothing about why.
//!
//! **A file that is already there and checks out is used as it is**, so a run
//! picked up after a Keeper stopped part way through downloads nothing twice.
//! What is downloaded goes to a `.part` beside it and is renamed into place
//! once it has been checked, so the name is only ever a whole file.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use sha2::{Digest, Sha256};

use super::Image;

/// `image`, downloaded into `into` and checked, handing how far it has got to
/// `progress` as it goes: the bytes so far, and the whole where the server said.
///
/// **Blocking**: the download is a runtime of its own on the calling thread,
/// which is the install run's.
pub fn fetched(
    image: &Image,
    into: &Path,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> anyhow::Result<PathBuf> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Verkstead couldn't start the download")?;

    runtime.block_on(fetching(image, into, progress))
}

async fn fetching(
    image: &Image,
    into: &Path,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> anyhow::Result<PathBuf> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("verkstead/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("Verkstead couldn't start the download")?;

    let expected = checksum(&client, image).await?;

    std::fs::create_dir_all(into)
        .with_context(|| format!("Verkstead couldn't make {}", into.display()))?;

    let file = into.join(image.file);

    if file.is_file() && digest_of(&file)? == expected {
        return Ok(file);
    }

    let part = into.join(format!("{}.part", image.file));

    let mut response = client
        .get(&image.url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .with_context(|| format!("Verkstead couldn't download {}", image.url))?;

    let whole = response.content_length();
    let mut written = std::fs::File::create(&part)
        .with_context(|| format!("Verkstead couldn't write {}", part.display()))?;
    let mut hasher = Sha256::new();
    let mut so_far = 0;

    progress(so_far, whole);

    while let Some(chunk) = response
        .chunk()
        .await
        .with_context(|| format!("The download of {} stopped part way", image.name))?
    {
        hasher.update(&chunk);
        written
            .write_all(&chunk)
            .with_context(|| format!("Verkstead couldn't write {}", part.display()))?;

        so_far += chunk.len() as u64;
        progress(so_far, whole);
    }

    drop(written);

    if hex(&hasher.finalize()) != expected {
        let _ = std::fs::remove_file(&part);

        bail!(
            "The {} image didn't match its checksum, so it was thrown away. Press Next to \
             download it again.",
            image.name
        );
    }

    std::fs::rename(&part, &file)
        .with_context(|| format!("Verkstead couldn't write {}", file.display()))?;

    Ok(file)
}

/// The SHA-256 the release says the image has, as lower-case hex.
async fn checksum(client: &reqwest::Client, image: &Image) -> anyhow::Result<String> {
    let said = client
        .get(&image.checksum)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .with_context(|| format!("Verkstead couldn't download {}", image.checksum))?
        .text()
        .await
        .with_context(|| format!("Verkstead couldn't download {}", image.checksum))?;

    let digest = said.split_whitespace().next().unwrap_or_default();

    if digest.len() != 64 || !digest.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("{} isn't a SHA-256 checksum.", image.checksum);
    }

    Ok(digest.to_ascii_lowercase())
}

/// The SHA-256 of the file at `path`, as lower-case hex.
fn digest_of(path: &Path) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Verkstead couldn't read {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 20];

    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("Verkstead couldn't read {}", path.display()))?;

        if read == 0 {
            return Ok(hex(&hasher.finalize()));
        }

        hasher.update(&buffer[..read]);
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A server for a suite, answering `GET /<name>` with what it was given and
/// anything else with a 404 — where the install suite downloads its image from.
#[cfg(test)]
pub(crate) mod serving {
    use std::collections::HashMap;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// The files, and how many times each was asked for.
    pub(crate) struct Serving {
        pub(crate) base: String,
        asked: Arc<Mutex<Vec<String>>>,
    }

    impl Serving {
        pub(crate) fn of(files: &[(&str, Vec<u8>)]) -> Serving {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let files: HashMap<String, Vec<u8>> = files
                .iter()
                .map(|(name, body)| (format!("/{name}"), body.clone()))
                .collect();
            let asked = Arc::new(Mutex::new(Vec::new()));

            std::thread::spawn({
                let asked = asked.clone();

                move || {
                    for stream in listener.incoming() {
                        let Ok(mut stream) = stream else { return };

                        let mut request = String::new();
                        let mut reader = BufReader::new(stream.try_clone().unwrap());
                        let _ = reader.read_line(&mut request);

                        // The rest of the head, which says nothing asked about.
                        loop {
                            let mut header = String::new();
                            if reader.read_line(&mut header).unwrap_or(0) <= 2 {
                                break;
                            }
                        }

                        let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
                        asked.lock().unwrap().push(path.clone());

                        let _ = match files.get(&path) {
                            Some(body) => stream
                                .write_all(
                                    format!(
                                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\
                                         Connection: close\r\n\r\n",
                                        body.len()
                                    )
                                    .as_bytes(),
                                )
                                .and_then(|()| stream.write_all(body)),
                            None => stream.write_all(
                                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\
                                  Connection: close\r\n\r\n",
                            ),
                        };
                    }
                }
            });

            Serving { base, asked }
        }

        /// The paths asked for, in order.
        pub(crate) fn asked(&self) -> Vec<String> {
            self.asked.lock().unwrap().clone()
        }
    }

    /// `body`'s SHA-256 as `sha256sum` prints it beside `name`.
    pub(crate) fn summed(body: &[u8], name: &str) -> Vec<u8> {
        use sha2::Digest;

        format!("{}  {name}\n", super::hex(&sha2::Sha256::digest(body))).into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::serving::{Serving, summed};
    use super::*;

    const BODY: &[u8] = b"a NixOS-WSL image, near enough";

    fn image(serving: &Serving) -> Image {
        Image {
            url: format!("{}/nixos.wsl", serving.base),
            checksum: format!("{}/nixos.wsl.sha256", serving.base),
            file: "nixos.wsl",
            name: "NixOS-WSL",
        }
    }

    #[test]
    fn an_image_is_downloaded_checked_and_put_in_place() {
        let serving = Serving::of(&[
            ("nixos.wsl", BODY.to_vec()),
            ("nixos.wsl.sha256", summed(BODY, "nixos.wsl")),
        ]);
        let dir = tempfile::tempdir().unwrap();

        let mut seen = Vec::new();
        let file = fetched(&image(&serving), dir.path(), &mut |so_far, whole| {
            seen.push((so_far, whole))
        })
        .unwrap();

        assert_eq!(file, dir.path().join("nixos.wsl"));
        assert_eq!(std::fs::read(&file).unwrap(), BODY);
        assert!(!dir.path().join("nixos.wsl.part").exists());
        assert_eq!(seen.first(), Some(&(0, Some(BODY.len() as u64))));
        assert_eq!(
            seen.last(),
            Some(&(BODY.len() as u64, Some(BODY.len() as u64)))
        );
    }

    #[test]
    fn an_image_already_there_and_whole_is_not_downloaded_again() {
        let serving = Serving::of(&[("nixos.wsl.sha256", summed(BODY, "nixos.wsl"))]);
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("nixos.wsl"), BODY).unwrap();

        fetched(&image(&serving), dir.path(), &mut |_, _| {}).unwrap();

        assert_eq!(serving.asked(), ["/nixos.wsl.sha256"]);
    }

    #[test]
    fn an_image_that_does_not_match_its_checksum_is_thrown_away() {
        let serving = Serving::of(&[
            ("nixos.wsl", b"something else".to_vec()),
            ("nixos.wsl.sha256", summed(BODY, "nixos.wsl")),
        ]);
        let dir = tempfile::tempdir().unwrap();

        let refused = fetched(&image(&serving), dir.path(), &mut |_, _| {}).unwrap_err();

        assert_eq!(
            refused.to_string(),
            "The NixOS-WSL image didn't match its checksum, so it was thrown away. Press Next \
             to download it again."
        );
        assert!(!dir.path().join("nixos.wsl").exists());
        assert!(!dir.path().join("nixos.wsl.part").exists());
    }
}
