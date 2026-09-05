//! The real Verkstead server, in the test process, and the CLI runs made
//! against it.
//!
//! Every round trip a `verkstead` verb makes is a real one: a server on a
//! runtime of its own, over a database on disk, with a Conversation for the
//! Sets to land on. So the fixture is shared rather than written per test file
//! — `ask` and `answers` are two halves of one Set's life and want the same
//! server under them.

use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Child, Output};
use std::time::{Duration, Instant};

use verkstead_schema::QuestionSet;
use verkstead_server::store::{self, StoredSet};

/// The Conversation these Sets are asked from, made by the server fixture over a
/// database with nothing in it — so it is always the first there is.
pub const ASKING_FROM: i64 = 1;

/// The real server, on a runtime of its own, so a blocking test can kill it
/// under the CLI's feet and bring it back on the same port.
pub struct Server {
    addr: SocketAddr,
    database: PathBuf,

    /// What a client is told the pipe beside the socket is called. Windows'
    /// own: there is no pipe to open anywhere else.
    #[cfg(windows)]
    pipe: String,

    runtime: tokio::runtime::Runtime,
}

impl Server {
    pub fn start(database: PathBuf) -> Self {
        Self::bind("127.0.0.1:0".parse().unwrap(), database)
    }

    pub fn bind(addr: SocketAddr, database: PathBuf) -> Self {
        Self::serve(addr, database)
    }

    fn serve(addr: SocketAddr, database: PathBuf) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();

        let (listener, addr, pool) = runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let addr = listener.local_addr().unwrap();
            let pool = verkstead_server::open_database(&database).await.unwrap();

            // Somewhere for the Sets to land. Every Set is asked from a
            // Conversation, and the base URL a session is given is what says
            // which — so a test standing in for a session has to be given the
            // same thing. Made only where there is none: this server is brought
            // up twice over one database, and the second time is a restart.
            if store::conversations(&pool).await.unwrap().is_empty() {
                let repo =
                    store::register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
                        .await
                        .unwrap()
                        .expect("nothing is registered at that path yet");

                let conversation = store::start_conversation(&pool, repo.id, "api-core-and-cli")
                    .await
                    .unwrap()
                    .expect("the Repo was just registered");
                assert_eq!(conversation, ASKING_FROM);
            }

            (listener, addr, pool)
        });

        // The pipe beside the socket, so that one test can put the round trip
        // through the pipe and the next through the URL against the same
        // server. Named after the database's own directory, which is the Data
        // Directory as far as the pipe is concerned — so a server brought back
        // up over the same database comes back on the same pipe as well as on
        // the same port. Opened on this runtime, because tokio's pipes register
        // with its reactor.
        #[cfg(windows)]
        let pipe = {
            let data_dir = database.parent().unwrap().to_owned();
            let pool = pool.clone();

            runtime.block_on(async move {
                let listener = verkstead_server::pipe::Listener::open(&data_dir, None)
                    .expect("nothing else holds this Data Directory's pipe");
                let spelling = listener.asked_through().to_owned();

                // The router built in here with it, because building one starts
                // the sweep the server starts, and that wants a runtime under
                // it.
                let served = verkstead_server::router(pool);
                tokio::spawn(async move {
                    let _ = axum::serve(listener, served).await;
                });

                spelling
            })
        };

        runtime.spawn(async move {
            let _ = axum::serve(listener, verkstead_server::router(pool)).await;
        });

        Server {
            addr,
            database,
            #[cfg(windows)]
            pipe,
            runtime,
        }
    }

    /// The database this server keeps its Sets in, for a test that wants to
    /// read one back through a pool of its own.
    pub fn database(&self) -> &Path {
        &self.database
    }

    /// Where the server is, whole — the viewer's namespace hangs off this, and
    /// it is nobody's Conversation.
    pub fn base(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// And what a session is given as `VERKSTEAD_SERVER`: the same server,
    /// scoped to the Conversation it is asking from.
    pub fn url(&self) -> String {
        format!("{}/conversations/{ASKING_FROM}", self.base())
    }

    /// And what a Windows session is given instead: the same server on the
    /// pipe beside its socket, scoped to the same Conversation.
    #[cfg(windows)]
    pub fn pipe_url(&self) -> String {
        format!("{}/conversations/{ASKING_FROM}", self.pipe)
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }

    /// Stop serving without a graceful shutdown, so a held long-poll is
    /// dropped exactly as it would be if the process were killed. Hands back
    /// what [`Server::bind`] needs to bring the same server up again.
    pub fn kill(self) -> (SocketAddr, PathBuf) {
        let Server {
            addr,
            database,
            runtime,
            ..
        } = self;
        runtime.shutdown_timeout(Duration::from_millis(100));
        (addr, database)
    }

    /// The Set the CLI submitted, read back through a second pool on the same
    /// file — the store is where the enriched Set can actually be seen.
    pub fn stored_set(&self, id: i64) -> Option<StoredSet> {
        self.block_on(async {
            let pool = verkstead_server::open_database(&self.database)
                .await
                .unwrap();
            let stored = store::load_set(&pool, id).await.unwrap();
            pool.close().await;
            stored
        })
    }

    /// Block until the CLI has submitted Set `id`, and hand back what it asked.
    ///
    /// The Set itself rather than the row holding it: a stored body this build
    /// cannot read is a broken test rather than a case with anything to say
    /// here.
    pub fn await_asked_set(&self, id: i64) -> QuestionSet {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(stored) = self.stored_set(id) {
                return stored
                    .set
                    .set()
                    .expect("the Set the CLI just sent reads back")
                    .clone();
            }
            assert!(
                Instant::now() < deadline,
                "the CLI never submitted Question Set {id}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Store a Set the way the server stores one asked on a backend whose
    /// sessions cannot wait, and hand back the id it went under.
    ///
    /// Written through the store rather than asked through the CLI, because
    /// which channel an ordinary ask is on is read off the agent type of the
    /// session that sent it — and there are no sessions here, this being the
    /// CLI's own suite. What a session on such a backend would have left behind
    /// is a Set stored with somebody idling on it, which is exactly this; that
    /// the server marks one so when a session of that type asks is
    /// `crates/server/tests/sessions.rs`'s to prove.
    pub fn store_and_nudge(&self, yaml: &str) -> i64 {
        let set = QuestionSet::from_yaml(yaml).expect("the fixture Set parses");

        self.block_on(async {
            let pool = verkstead_server::open_database(&self.database)
                .await
                .unwrap();
            let created = store::ask(&pool, ASKING_FROM, &set, store::Ask::StoreAndNudge)
                .await
                .unwrap()
                .expect("the fixture's Conversation is there to ask from");
            pool.close().await;
            created.id
        })
    }

    /// Answer a Set the way the human's device does: YAML over HTTP.
    pub fn answer(&self, id: i64, yaml: &str) {
        let reply = ureq::post(format!("{}/api/v1/sets/{id}/response", self.url()))
            .header("Content-Type", "application/yaml")
            .send(yaml)
            .unwrap();
        assert_eq!(reply.status().as_u16(), 201);
    }

    /// Lock a Set unanswered the way the human's browser does. It lives in the
    /// viewer's namespace and nowhere else: the agent API has no route for it,
    /// because only a human may close a Set nobody is going to answer.
    pub fn lock(&self, id: i64) {
        let reply = ureq::post(format!("{}/api/ui/sets/{id}/lock", self.base()))
            .header("Content-Type", "application/json")
            .send("{}")
            .unwrap();
        assert_eq!(reply.status().as_u16(), 200);
    }
}

/// What the CLI wrote and how it exited, insisting it exited at all.
pub fn finished(child: Child) -> Output {
    let output = child.wait_with_output().unwrap();
    eprintln!(
        "verkstead stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}
