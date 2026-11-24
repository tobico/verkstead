//! GitHub's component: every operation the server asks of a Forge, written
//! against `wit/plugin.wit`'s `forge` world (ADR-0030, *Forge*). Everything
//! else about GitHub — the hosts it answers for, the noun, how git in the
//! Sandbox signs in, the tool a session is lent, what it can do — is data, in
//! `plugin.yaml` beside this crate.
//!
//! **It speaks to GitHub itself.** REST and GraphQL over WASI HTTP, to the
//! `api.github.com` the manifest asks for as its one outbound host and to
//! whatever host a self-hosted instance was added as — see [`api`], where
//! which of the two APIs answers which question is written down, and [`http`],
//! which is the whole of how a request leaves here.
//!
//! **The token is the host's to set.** It arrives in the environment under the
//! variable the manifest's `secrets` capability names, for the one call it was
//! set for, so no operation here takes one and nothing of one is ever answered
//! back. An environment with none is `not-logged-in`.
//!
//! **And the readings are carried over rather than re-decided** — GitHub's
//! word for a check, for whether a pull request merges, for where it has got
//! to, and what a URL says — so a Repo reads the same across the boundary as
//! it did in process. See [`names`] for the string work and [`api`] for the
//! rest.

// The generated lowering of a record with seven fields is a function with
// seven arguments, which is the world's shape rather than anything written
// here.
#![allow(clippy::too_many_arguments)]

wit_bindgen::generate!({
    path: "../../crates/server/wit",
    world: "forge",
});

mod api;
mod http;
mod names;

struct Github;

impl Guest for Github {
    fn verify() -> Result<Account, Trouble> {
        api::verify()
    }

    fn pull_request(remote: String, branch: String) -> Result<Found, Trouble> {
        api::pull_request(&remote, &branch)
    }

    fn pull_request_numbered(remote: String, number: i64) -> Result<Numbered, Trouble> {
        api::numbered(&remote, number)
    }

    fn open_pull_request(remote: String, opening: Opening) -> Result<Numbered, Trouble> {
        api::open_pull_request(&remote, &opening)
    }

    fn open_pull_requests(remote: String) -> Result<Vec<Numbered>, Trouble> {
        api::open_pull_requests(&remote)
    }

    fn details(remote: String, number: i64) -> Result<Details, Trouble> {
        api::details(&remote, number)
    }

    fn checks(remote: String, number: i64) -> Result<Suite, Trouble> {
        api::checks(&remote, number)
    }

    fn landing(remote: String, number: i64) -> Result<Landing, Trouble> {
        api::landing(&remote, number)
    }

    fn comments(remote: String, number: i64) -> Result<Vec<Comment>, Trouble> {
        api::comments(&remote, number)
    }

    fn comment(remote: String, number: i64, body: String) -> Result<String, Trouble> {
        api::comment(&remote, number, &body)
    }

    fn publish(description: String, name: String, content: String) -> Result<Published, Trouble> {
        api::publish(&description, &name, &content)
    }

    fn unpublish(id: String) -> Result<(), Trouble> {
        api::unpublish(&id)
    }

    fn create_repository(name: String) -> Result<Made, Trouble> {
        api::create_repository(&name)
    }

    fn lacks_scope(said: String) -> bool {
        names::unscoped(&said)
    }

    fn target_in(prose: String) -> Option<NamedPullRequest> {
        names::pull_request_in(&prose).map(named)
    }

    fn target_named(target: String) -> Option<NamedPullRequest> {
        names::pull_request_named(&target).map(named)
    }

    fn repository_in(url: String) -> Option<String> {
        names::repository_in(&url)
    }

    fn repository_of(origin: String) -> Option<String> {
        names::dot_com_repository(&origin)
    }
}

/// A pull request a name said, as the world holds one.
fn named(named: names::Named) -> NamedPullRequest {
    NamedPullRequest {
        number: named.number,
        repository: named.repository,
    }
}

export!(Github);
