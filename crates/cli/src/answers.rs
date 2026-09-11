//! `verkstead answers`: come back for the Response to a Set stored earlier.
//!
//! The blocking ask hands a session its Answers by never returning until they
//! are there. A session that stored its Set and ended its turn instead has to
//! be able to come back for them, and this is how — one id in, one Response
//! out, in the shape the blocking ask would have printed.
//!
//! **And a blocking ask whose wait was killed comes back the same way.** The
//! wait is a shell command a harness is running in the background, and a harness
//! may stop one; what that leaves is a Set answered or about to be, and a
//! session with no wait in front of it. Nothing about the Set changed, so
//! nothing here needs to know which of the two brought a session to the door —
//! the id is the whole of what it asks for, and `verkstead ask` says the id as
//! it opens the wait for exactly this reason.
//!
//! A fetch rather than a wait. Nothing here reconnects and nothing holds: the
//! command polls once, and a Set nobody has answered yet is a refusal rather
//! than something to idle on. What tells a session the Answers are there is the
//! nudge, not this command sitting on the door — and the nudge reaches a wait
//! that was killed too, once the server has seen that nothing took the Response.

use anyhow::{Context, Result};

use crate::client::Client;

/// Print Set `id`'s Response on stdout, or fail saying why there is none.
pub fn answers(id: i64, server: &str) -> Result<()> {
    let response = Client::new(server)?.fetch(id)?;

    crate::ask::deliver(
        response
            .to_yaml()
            .context("rendering the Response as YAML")?,
    )
}
