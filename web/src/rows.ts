//! A row that wraps its control in a `<label>`, and the press it answers to.
//!
//! The pattern is everywhere in the app: a checkbox or a radio, the words that
//! name it beside it, and a `<label>` round the pair so the whole row is one
//! thing to press. What made that true was the browser's own doing — a label
//! activates its control by forwarding its click to it — and **the browser
//! withholds that forwarding the moment the pointer moved at all** between the
//! press and the release.
//!
//! Which is not the edge case it sounds like. A row of words is pressed by hands
//! that are not holding perfectly still, and the gesture that slid a pixel used
//! to select one character of the words and do nothing else: the one way a row
//! has of being pressed was also the one way to miss it. It was reported of a
//! Question Set's answers, where missing it is expensive, and it was every row
//! in the app.
//!
//! So a row answers its own click instead — [`rowPress`] is that handler, in one
//! place because it is one arrangement, and the rows that use it read the same
//! way whether they answer a Question, flip a setting or steer a Conversation.
//!
//! **The other half of it is in each row's stylesheet**: a row is a press, so it
//! takes no selection and says `cursor: pointer`. Between them the gesture can
//! only be a press, which is what a row was always offering to be.

/// The press a row that wraps its control answers to: the row's own click,
/// counted once.
///
/// Hung on the element that draws the row — the `<label>`, or the `<tr>` of a
/// table where there is no label to wrap a row in.
///
/// Three things happen for one press:
///
/// - **The forwarding is cancelled**, so the gesture cannot also arrive as the
///   control's own click and be counted twice. A still press forwards and a slid
///   one does not, and cancelling is what makes the two the same.
/// - **The control is focused**, because the forwarding is what used to do that
///   and cancelling it took that as well. Focus moved by hand after a press
///   draws no `:focus-visible` ring, so a press looks exactly as it did.
/// - **`act` is called**, which is the row's own meaning and the caller's to
///   write: what a second press on the same row does is a question only the
///   caller can answer, and they differ — an Option clears, a checkbox flips, a
///   radio that is one of a set stands where it is.
///
/// A press that landed on the control itself is left alone: the control's own
/// handlers have it, that press is also how the keyboard picks, and a forwarded
/// click is indistinguishable from one here — which is why the forwarding is
/// cancelled rather than told apart.
///
/// **A control that will not take a press takes none from the row either.** The
/// browser refuses a disabled control whatever a label forwards it, and a row
/// that answered for itself would otherwise be a way round the refusal.
///
/// **And the row stands out of a press that landed on something interactive
/// inside it** — see [`interactive`]. Cancelling the forwarding cancels every
/// other default the press carried with it, and a link in an Option's words is
/// one of those: the whole of what the row does is skipped there, so the link
/// opens the way it always did.
export function rowPress(
  act: () => void,
): (event: MouseEvent & { currentTarget: HTMLElement }) => void {
  return (event) => {
    const control = event.currentTarget.querySelector("input");

    if (
      event.target === control ||
      control?.disabled ||
      interactive(event.target, event.currentTarget)
    ) {
      return;
    }

    event.preventDefault();
    control?.focus();
    act();
  };
}

/// Whether the press landed on something inside the row that answers a press of
/// its own — a link above all: an Option's words are the agent's markdown, and a
/// link is one of the marks the server keeps in it.
///
/// This is the browser's own rule about a `<label>`, which forwards nothing for
/// a press on interactive content inside it, or on anything inside that. So a
/// link in a row worked before the row began answering its own click, and
/// skipping the whole handler is what keeps it working — standing out of one
/// cannot count a gesture twice, there being no forwarded click to cancel.
///
/// The row itself is never asked: a `<label>` is interactive content, and a row
/// that ruled itself out would answer nothing at all.
function interactive(landed: EventTarget | null, row: HTMLElement): boolean {
  for (
    let element = landed instanceof Element ? landed : null;
    element !== null && element !== row;
    element = element.parentElement
  ) {
    if (element.matches(INTERACTIVE)) {
      return true;
    }
  }

  return false;
}

/// The interactive content of the HTML standard, as a selector: the elements a
/// `<label>` refuses to forward a press from.
///
/// Written out whole rather than trimmed to what the rows hold today, because
/// what a row holds is the caller's and the next one may hold any of it. An `a`
/// or an `area` with no `href` is not interactive and is not here, which is the
/// standard's own line.
const INTERACTIVE = [
  "a[href]",
  "area[href]",
  "audio[controls]",
  "button",
  "details",
  "embed",
  "iframe",
  "img[usemap]",
  "input",
  "label",
  "select",
  "textarea",
  "video[controls]",
].join(", ");
