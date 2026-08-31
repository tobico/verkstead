//! `verkstead answers`: come back for the Response to a Set stored earlier.
//!
//! The blocking ask hands a session its Answers by never returning until they
//! are there. A session that stored its Set and ended its turn instead has to
//! be able to come back for them, and this is how — one id in, one Response
//! out, in the shape the blocking ask would have printed.
//!
//! A fetch rather than a wait. Nothing here reconnects and nothing holds: the
//! command polls once, and a Set nobody has answered yet is a refusal rather
//! than something to idle on. What tells a session the Answers are there is the
//! nudge, not this command sitting on the door.

use anyhow::{Context, Result};

use crate::client::Client;

/// Print Set `id`'s Response on stdout, or fail saying why there is none.
pub fn answers(id: i64, server: &str) -> Result<()> {
    let response = Client::new(server).fetch(id)?;

    crate::ask::deliver(
        response
            .to_yaml()
            .context("rendering the Response as YAML")?,
    )
}
