//! Sharing a Conversation: the one self-contained file a colleague opens.
//!
//! What travels is the share build of the viewer — the same SPA, built to one
//! HTML file with its script and its stylesheets inlined
//! (`web/vite.share.config.ts`) — with the Conversation's own record put into it
//! on the way out. It fetches nothing and talks to nothing: everything it draws
//! is in the file, so it opens off a disk with the server stopped and reads the
//! same as it does on the tailnet.
//!
//! Two halves, and the seam between them is a pair of slots. The build leaves
//! them empty in the document and this fills them in.
//!
//! The first holds the record — a `<script type="application/json">` holding
//! `null` until the Conversation goes in. The alternative was a viewer that
//! fetched its own payload from somewhere, which is the one thing a file sent as
//! an attachment cannot do. What goes in it is
//! [`verkstead_render::SharedConversation`], which is where the curation is:
//! what boards a share, and what is taken off it, is a rendering decision and is
//! made once, over there.
//!
//! The second holds mermaid, and is empty on almost every share. The renderer is
//! the one thing on a Set's page the browser draws for itself, and it is three
//! megabytes — so it rides in the file only where a Set in the record carries a
//! Diagram, and a Conversation nobody drew a picture in stays the size of its own
//! record.

use serde::Serialize;
use time::OffsetDateTime;
use verkstead_render::{ConversationView, Lifecycle, SharedConversation};

/// The empty record slot the share build leaves in the document, exactly as it
/// writes it.
///
/// `null` rather than nothing, so that the built template is a page that opens:
/// a share with no Conversation in it says so in its own words rather than dying
/// on a parse. Nothing else in the document has this id, and vite leaves a
/// script of an unknown type alone.
const RECORD: &str = r#"<script id="share" type="application/json">"#;

/// And the empty one beside it, where the diagram renderer goes on the shares
/// that need one.
///
/// Empty in the template and empty in most shares. Mermaid is three megabytes,
/// and a Conversation nobody drew a picture in would be twenty times the size of
/// its own record if the library rode along regardless — so what fills this is
/// decided per share, from the Sets in the bundle. See `web/src/share/mermaid.ts`
/// for the other side of it.
const DIAGRAMS: &str = r#"<script id="diagrams">"#;

/// And where either ends, which is what the contents are written between.
const CLOSES: &str = "</script>";

/// The share file: the built template with one Conversation's record in it, and
/// the diagram renderer where the record needs one.
///
/// `None` where the template has neither slot, which is a viewer built by
/// something that is not this build — the endpoint says so rather than handing
/// over a page that would draw nothing.
///
/// Written against anything that serializes rather than against the bundle
/// alone, because what this does is put JSON where the slot was: the shape of
/// the payload is [`verkstead_render::shared`]'s business and none of this
/// function's.
pub(crate) fn file<Bundle: Serialize>(
    template: &str,
    bundle: &Bundle,
    renderer: Option<&str>,
) -> Option<String> {
    let filled = fill(template, RECORD, &bundled(bundle)?)?;

    match renderer {
        Some(mermaid) => fill(&filled, DIAGRAMS, &inline(mermaid)),
        None => Some(filled),
    }
}

/// One slot filled: whatever the template had between that opening tag and the
/// next close, replaced by what is being put there.
fn fill(document: &str, opens: &str, contents: &str) -> Option<String> {
    let at = document.find(opens)?;
    let empty = at + opens.len();
    let closes = empty + document[empty..].find(CLOSES)?;

    Some(format!(
        "{}{}{}",
        &document[..empty],
        contents,
        &document[closes..]
    ))
}

/// Whether a share carries the diagram renderer, which is a fact about what is
/// in it: any Set with a Diagram on it, and none if there is none.
///
/// Asked of the rendered Sets rather than of their markup, because the rendering
/// already answered it — `SetView::diagrams` is what tells the live page whether
/// to fetch mermaid at all, and this is the same question asked of a page that
/// cannot fetch.
pub(crate) fn diagrammed(bundle: &SharedConversation) -> bool {
    bundle.sets.iter().any(|set| set.diagrams)
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

/// And compiled JavaScript as it may be written inside one.
///
/// The same hazard and a narrower escape, because this is code rather than data:
/// a `</script` in a string or a regular expression would end the block early,
/// and `<\/script` means the same thing to JavaScript and nothing to the parser
/// looking for the end of the tag. The share build escapes its own chunk exactly
/// this way — see `inlineScript` in `web/vite.share.config.ts`.
fn inline(code: &str) -> String {
    const ENDS: &str = "</script";

    let mut safe = String::with_capacity(code.len());
    let mut from = 0;

    for at in code.match_indices('<').map(|(at, _)| at) {
        let ends = at + ENDS.len();

        if ends <= code.len() && code.as_bytes()[at..ends].eq_ignore_ascii_case(ENDS.as_bytes()) {
            safe.push_str(&code[from..at]);
            // The escape, and then the tag exactly as it was written: `<\/SCRIPT`
            // is as good an escape as `<\/script`, and rewriting the case would
            // be changing what a string in the renderer holds.
            safe.push_str("<\\");
            safe.push_str(&code[at + 1..ends]);
            from = ends;
        }
    }

    safe.push_str(&code[from..]);
    safe
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
    /// this module has anything to say about: the two slots, and something
    /// either side of them that has to survive.
    const TEMPLATE: &str = concat!(
        r#"<!doctype html><html><head>"#,
        r#"<script id="share" type="application/json">null</script>"#,
        r#"<script id="diagrams"></script>"#,
        r#"</head><body><div id="app"></div>"#,
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
        let filled = file(TEMPLATE, &Bundle { html: "hello" }, None).unwrap();

        assert!(
            filled.contains(
                r#"<script id="share" type="application/json">{"html":"hello"}</script>"#
            )
        );

        // And the document around it is untouched, script that runs and all.
        assert!(filled.starts_with(r#"<!doctype html><html><head>"#));
        assert!(filled.ends_with("<script>booted()</script></body></html>"));
    }

    #[test]
    fn nothing_in_the_record_can_end_the_block() {
        let filled = file(
            TEMPLATE,
            &Bundle {
                html: "</script><script>steal()</script>",
            },
            None,
        )
        .unwrap();

        // The document has the three scripts it was built with and no fourth:
        // what was written is inside the JSON, spelled with escapes.
        assert_eq!(filled.matches("<script").count(), 3);
        assert!(filled.contains(r"</script>"));
    }

    #[test]
    fn a_share_with_no_diagrams_carries_no_renderer() {
        let filled = file(TEMPLATE, &Bundle { html: "hello" }, None).unwrap();

        // The slot is left exactly as the build wrote it: empty, and there for
        // the next share to fill.
        assert!(filled.contains(r#"<script id="diagrams"></script>"#));
    }

    #[test]
    fn a_share_with_diagrams_carries_it_inside_the_document() {
        let filled = file(
            TEMPLATE,
            &Bundle { html: "hello" },
            Some("window.verksteadMermaid = renderer;"),
        )
        .unwrap();

        assert!(
            filled
                .contains(r#"<script id="diagrams">window.verksteadMermaid = renderer;</script>"#)
        );

        // And the record is still where it was: the two slots are filled
        // independently, and one does not read the other's contents.
        assert!(filled.contains(r#"<script id="share" type="application/json">{"html":"hello"}"#));
    }

    #[test]
    fn nothing_in_the_renderer_can_end_the_block_either() {
        let filled = file(
            TEMPLATE,
            &Bundle { html: "hello" },
            // Which mermaid really does carry: it writes markup, so its own
            // source has closing tags in strings.
            Some(r#"const shut = "</SCRIPT>"; const also = "</script>";"#),
        )
        .unwrap();

        // The three the template was built with and no fourth: the renderer went
        // inside the slot that was already there, and nothing its strings hold
        // opened one of its own.
        assert_eq!(filled.matches("<script").count(), 3);
        assert!(filled.contains(r#"const shut = "<\/SCRIPT>"; const also = "<\/script>";"#));
    }

    #[test]
    fn a_template_with_no_slot_is_no_template() {
        assert!(file("<!doctype html><html></html>", &Bundle { html: "" }, None).is_none());
    }

    #[test]
    fn a_template_with_nowhere_to_put_the_renderer_is_no_template_either() {
        const NO_SLOT: &str = concat!(
            r#"<!doctype html><html><head>"#,
            r#"<script id="share" type="application/json">null</script>"#,
            "</head><body></body></html>",
        );

        assert!(file(NO_SLOT, &Bundle { html: "" }, None).is_some());
        assert!(file(NO_SLOT, &Bundle { html: "" }, Some("renderer")).is_none());
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
