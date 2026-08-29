//! Sharing a Conversation: the one self-contained file a colleague opens.
//!
//! What travels is the share build of the viewer — the same SPA, built to one
//! HTML file with its script and its stylesheets inlined
//! (`web/vite.share.config.ts`) — with the Conversation's own record put into it
//! on the way out. It fetches nothing and talks to nothing: everything it draws
//! is in the file, so it opens off a disk with the server stopped and reads the
//! same as it does on the tailnet.
//!
//! Two halves, and the seam between them is a slot. The build leaves an empty
//! one in the document — a `<script type="application/json">` holding `null` —
//! and this puts the bundle there. The alternative was a viewer that fetched its
//! own payload from somewhere, which is the one thing a file sent as an
//! attachment cannot do.
//!
//! What goes in the slot is [`verkstead_render::SharedConversation`], which is
//! where the curation is: what boards a share, and what is taken off it, is a
//! rendering decision and is made once, over there.

use serde::Serialize;
use time::OffsetDateTime;
use verkstead_render::{ConversationView, Lifecycle};

/// The empty slot the share build leaves in the document, exactly as it writes
/// it.
///
/// `null` rather than nothing, so that the built template is a page that opens:
/// a share with no Conversation in it says so in its own words rather than dying
/// on a parse. Nothing else in the document has this id, and vite leaves a
/// script of an unknown type alone.
const OPENS: &str = r#"<script id="share" type="application/json">"#;

/// And where it ends, which is what the bundle is written between.
const CLOSES: &str = "</script>";

/// The share file: the built template with one Conversation's record in it.
///
/// `None` where the template has no slot, which is a viewer built by something
/// that is not this build — the endpoint says so rather than handing over a page
/// that would draw nothing.
///
/// Written against anything that serializes rather than against the bundle
/// alone, because what this does is put JSON where the slot was: the shape of
/// the payload is [`verkstead_render::shared`]'s business and none of this
/// function's.
pub(crate) fn file<Bundle: Serialize>(template: &str, bundle: &Bundle) -> Option<String> {
    let opens = template.find(OPENS)?;
    let empty = opens + OPENS.len();
    let closes = empty + template[empty..].find(CLOSES)?;

    Some(format!(
        "{}{}{}",
        &template[..empty],
        bundled(bundle)?,
        &template[closes..]
    ))
}

/// The bundle as it may be written inside a `<script>`.
///
/// JSON and one substitution: `<` becomes its escape. A `</script>` anywhere in
/// what an agent wrote — and agents write about HTML — would otherwise end the
/// block early and leave the rest of the record loose in the document. Safe over
/// the whole text because JSON has no `<` of its own: every one of them is
/// inside a string, where the escape means the same character.
fn bundled<Bundle: Serialize>(bundle: &Bundle) -> Option<String> {
    Some(serde_json::to_string(bundle).ok()?.replace('<', "\\u003c"))
}

/// What the file is called when it lands in somebody's downloads: the
/// Conversation, and the day the share was taken.
///
/// The branch, because that is what a Conversation is called wherever it is
/// named — and `draft` where nobody has named one, which is the same answer the
/// sidebar and the pane header give. The date because a share is a snapshot: two
/// of them are two files, and a reader holding both should be able to tell which
/// is the later without opening either.
pub(crate) fn filename(branch: &str, named: bool, at: OffsetDateTime) -> String {
    let stem = if named { plain(branch) } else { String::new() };
    let stem = if stem.is_empty() {
        "draft".to_owned()
    } else {
        stem
    };

    format!(
        "{stem}-{:04}-{:02}-{:02}.html",
        at.year(),
        u8::from(at.month()),
        at.day()
    )
}

/// Whether the branch this Conversation carries is a name somebody settled on,
/// rather than the one Verkstead invented to have something to cut.
///
/// The rule the viewer titles a Conversation by, said here because the file is
/// titled the same way: named by the human, or a Conversation past drafting
/// whose first session is no longer the one to rename it. Anything else is a
/// Draft, whatever the record is carrying — see `titled` in
/// `web/src/workbench/naming.ts`, which is the same rule at the other end of the
/// wire.
pub(crate) fn settled(conversation: &ConversationView) -> bool {
    conversation.branch_named || (conversation.state != Lifecycle::Draft && !conversation.naming)
}

/// A branch name as a filename may hold it: letters, digits and the three marks
/// that read as part of a word, with everything else — the slashes above all —
/// standing as a hyphen.
///
/// A name is a name rather than a path, and the `filename=` of a header is one
/// segment: what the human sees in their downloads should be the branch they
/// know, spelled with nothing in it that another program has an opinion about.
fn plain(branch: &str) -> String {
    let mut plain = String::with_capacity(branch.len());

    for character in branch.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
            plain.push(character);
        } else if !plain.ends_with('-') {
            plain.push('-');
        }
    }

    plain.trim_matches(['-', '.']).to_owned()
}

#[cfg(test)]
mod tests {
    use time::format_description::well_known::Rfc3339;

    use super::*;

    /// A template of the shape the share build writes, cut down to the parts
    /// this module has anything to say about: the slot, and something either
    /// side of it that has to survive.
    const TEMPLATE: &str = concat!(
        r#"<!doctype html><html><body><div id="app"></div>"#,
        r#"<script id="share" type="application/json">null</script>"#,
        "<script>booted()</script></body></html>",
    );

    /// A bundle standing in for the real one. What this module does to a bundle
    /// is serialize it, so a shape with the one hazard in it — a closing tag in
    /// something an agent wrote — says more here than a whole Conversation
    /// would.
    #[derive(Serialize)]
    struct Bundle {
        html: &'static str,
    }

    #[test]
    fn the_bundle_goes_where_the_slot_was() {
        let filled = file(TEMPLATE, &Bundle { html: "hello" }).unwrap();

        assert!(
            filled.contains(
                r#"<script id="share" type="application/json">{"html":"hello"}</script>"#
            )
        );

        // And the document around it is untouched, script that runs and all.
        assert!(filled.starts_with(r#"<!doctype html><html><body><div id="app"></div>"#));
        assert!(filled.ends_with("<script>booted()</script></body></html>"));
    }

    #[test]
    fn nothing_in_the_record_can_end_the_block() {
        let filled = file(
            TEMPLATE,
            &Bundle {
                html: "</script><script>steal()</script>",
            },
        )
        .unwrap();

        // The document has the two scripts it was built with and no third: what
        // was written is inside the JSON, spelled with escapes.
        assert_eq!(filled.matches("<script").count(), 2);
        assert!(filled.contains(r"</script>"));
    }

    #[test]
    fn a_template_with_no_slot_is_no_template() {
        assert!(file("<!doctype html><html></html>", &Bundle { html: "" }).is_none());
    }

    #[test]
    fn a_branch_is_spelled_as_one_segment() {
        assert_eq!(
            filename("sharing", true, stamp()),
            "sharing-2026-08-30.html"
        );
        assert_eq!(
            filename("feature/one two", true, stamp()),
            "feature-one-two-2026-08-30.html"
        );
    }

    #[test]
    fn a_conversation_nobody_named_is_a_draft() {
        assert_eq!(
            filename("verkstead-4821", false, stamp()),
            "draft-2026-08-30.html"
        );
    }

    fn stamp() -> OffsetDateTime {
        OffsetDateTime::parse("2026-08-30T01:02:03Z", &Rfc3339).unwrap()
    }
}
