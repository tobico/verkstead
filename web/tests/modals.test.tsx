//! The one modal, tested where it lives rather than once per form drawn in it.
//!
//! What every modal in the UI owes the human is here — it is a real `dialog`
//! opened as a modal, Escape takes it back, a press away from the card takes it
//! back, and nothing of it is on the page while it is shut — and it is asserted
//! against `Modal` itself, because that is the only place any of it is written.
//! What the callers' own suites carry is the half that is theirs: what is inside
//! the card, and that closing it did not save.
//!
//! `tests/setup.ts` fills in the `dialog` methods jsdom has none of. So what is
//! read here is that the component asks the platform for the right things and
//! answers what the platform says back — the platform's own half of it is the
//! browser's to keep.

import { fireEvent, render, waitFor } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { describe, expect, it } from "vitest";

import { Modal } from "../src/Modal";
// The one modal, both ways: the hashed names to query the page by, and the
// source to read the rules off, jsdom laying nothing out to read them from.
import modal from "../src/Modal.module.css";
import stylesheet from "../src/Modal.module.css?raw";
// The Set page's own vocabulary, where the two confirm sheets are, for the half
// of this that is about what they no longer draw for themselves.
import sheets from "../src/set/Sheet.module.css?raw";

/// A modal that starts open, with the way it was closed recorded.
///
/// `insist` is the one card nobody opened — see the two tests about it below,
/// and `src/Joining.tsx`, which is why the flag exists.
function mount(insist = false): {
  container: HTMLElement;
  open: () => boolean;
  shut: () => void;
} {
  const [open, setOpen] = createSignal(true);

  const { container } = render(() => (
    <Modal
      class="example"
      open={open()}
      close={() => setOpen(false)}
      labelledBy="example-title"
      insist={insist}
    >
      <p id="example-title">Are you sure?</p>
      <button type="button">Do the thing</button>
    </Modal>
  ));

  return { container, open, shut: () => setOpen(false) };
}

/// The dialog, or nothing where the modal is shut.
function sheet(container: ParentNode): HTMLDialogElement | null {
  return container.querySelector<HTMLDialogElement>(`dialog.${modal.modal}`);
}

describe("a modal", () => {
  /// A native `dialog`, opened as one: everything a modal owes the human — the
  /// top layer, the backdrop, the page behind going inert, the focus — is the
  /// platform's, and asking for it this way is the whole of how it is had.
  it("draws a dialog and opens it modally", () => {
    const { container } = mount();

    const dialog = sheet(container)!;
    expect(dialog.tagName).toBe("DIALOG");
    expect(dialog.open).toBe(true);
    expect(dialog.getAttribute("aria-labelledby")).toBe("example-title");
    expect(dialog.classList).toContain("example");
  });

  /// The caller's contents, in the card that carries the padding — which the
  /// dialog itself must not, so that a press on the dialog is unambiguously a
  /// press away from the card.
  it("puts the caller's contents in a card of its own", () => {
    const { container } = mount();

    const card = sheet(container)!.querySelector(`.${modal.card}`)!;
    expect(card.querySelector("#example-title")!.textContent).toBe(
      "Are you sure?",
    );
  });

  /// Shut, nothing of it is on the page: not hidden, not inert, not there. Which
  /// is what a form built afresh from the row it was opened beside depends on.
  it("draws nothing at all while it is shut", () => {
    const [open, setOpen] = createSignal(false);
    const { container } = render(() => (
      <Modal class="example" open={open()} close={() => setOpen(false)}>
        <p>Are you sure?</p>
      </Modal>
    ));

    expect(sheet(container)).toBeNull();

    setOpen(true);
    expect(sheet(container)).toBeTruthy();
  });

  /// The way out that needs no aim, which is the platform's and is answered
  /// here: the dialog closes itself, and the caller is told so it can put away
  /// whatever it was holding open beside it.
  it("tells the caller when Escape closed it", async () => {
    const { container, open } = mount();

    fireEvent.keyDown(document, { key: "Escape" });

    await waitFor(() => expect(open()).toBe(false));
    expect(sheet(container)).toBeNull();
  });

  /// The one way out `dialog` has no opinion about. A press on the backdrop is
  /// reported as a press on the dialog itself, and this is where that is turned
  /// into a close.
  it("closes on a press away from the card", async () => {
    const { container, open } = mount();

    fireEvent.click(sheet(container)!);

    await waitFor(() => expect(open()).toBe(false));
    expect(sheet(container)).toBeNull();
  });

  /// Except on the one card nobody opened. A join arriving from another machine
  /// raises a modal over whatever the human was reading, so there is nowhere for
  /// Escape to go back to — and a question pressed away from would run out ten
  /// minutes later with nothing on the page ever having said it was asked.
  ///
  /// Refused at `cancel`, which is what Escape fires and is cancelable: the
  /// platform's own way of saying a dialog has no way out but its contents,
  /// rather than the key being swallowed somewhere up the page.
  it("stays up under Escape where the card insists on an answer", () => {
    const { container, open } = mount(true);

    fireEvent.keyDown(document, { key: "Escape" });

    expect(open()).toBe(true);
    expect(sheet(container)!.open).toBe(true);
  });

  /// And nor is a press away from it a way out of that one, for the same reason.
  it("stays up under a press on the backdrop where it insists", () => {
    const { container, open } = mount(true);

    fireEvent.click(sheet(container)!);

    expect(open()).toBe(true);
    expect(sheet(container)!.open).toBe(true);
  });

  /// And a press inside the card is not one: it is the whole of what a form in a
  /// modal is for.
  it("stays open under a press on the card", () => {
    const { container, open } = mount();

    fireEvent.click(sheet(container)!.querySelector("button")!);

    expect(open()).toBe(true);
    expect(sheet(container)).toBeTruthy();
  });

  it("goes away when the caller says it is shut", async () => {
    const { container, shut } = mount();

    shut();

    await waitFor(() => expect(sheet(container)).toBeNull());
  });

  /// And says nothing back about it, however the caller got there.
  ///
  /// Taking the modal away closes the `dialog` on the way out, so that the focus
  /// goes back to whatever opened it — and a `dialog` closed fires its own close
  /// event on a node that is leaving the document but still carries its
  /// listeners. Said back, that is the caller being told a second time about a
  /// close it made itself and has already acted on: harmless where a `close`
  /// only puts a signal back, and a second Repo landing on a draft where it does
  /// more.
  it("does not say a close the caller made back to them", async () => {
    const [open, setOpen] = createSignal(true);
    const closed: string[] = [];

    /// A caller whose way out of its own card is what unmounts the card — which
    /// is the Create repo modal's, where the Repo it made goes onto the draft on
    /// the way past.
    const leave = () => {
      closed.push("leave");
      setOpen(false);
    };

    const { container } = render(() => (
      <Modal class="example" open={open()} close={leave} name="example">
        <button type="button" onClick={leave}>
          Done
        </button>
      </Modal>
    ));

    fireEvent.click(container.querySelector("button")!);

    await waitFor(() => expect(sheet(container)).toBeNull());
    // Waited past the turn a close event would land on, so that a second saying
    // has had every chance to arrive rather than merely not having arrived yet.
    await new Promise((go) => setTimeout(go, 0));

    expect(closed).toEqual(["leave"]);
  });
});

/// One rule rather than one per modal, which is the visible half of there being
/// one modal: what is drawn over the page looks the same wherever it was opened
/// from.
describe("what every modal is drawn with", () => {
  /// The dialog is the frame and the card is the box. A press on the backdrop is
  /// reported as a press on the dialog, so any padding there would make a press
  /// on the card's own margin read as a press away from the card.
  it("keeps the padding on the card and off the dialog", () => {
    expect(block(".modal")).toContain("padding: 0;");
    expect(block(".card")).toContain("padding: 1.25rem;");
  });

  it("dims the page behind it", () => {
    expect(block(".modal::backdrop")).toContain("background: rgb(0 0 0 / 45%)");
  });

  /// Rising from the bottom edge where a thumb is, and centred once the window
  /// is wider than a phone.
  it("rises from the bottom edge until the window is wider than a phone", () => {
    expect(block(".modal")).toContain("margin: auto auto 1rem;");
    expect(stylesheet).toContain(
      "@media (min-width: 30rem) {\n  .modal {\n    margin: auto;\n  }\n}",
    );
  });

  /// The point of one component: no sheet drawn over the page carries a backdrop
  /// or a box of its own to drift away from the shared one.
  it("leaves the confirm sheets nothing of their own to draw", () => {
    expect(sheets).not.toContain(".confirm-backdrop");
    expect(sheets).not.toContain("\n.confirm {\n");
  });
});

/// What one rule declares, read off the stylesheet by the selector that carries
/// it.
function block(selector: string): string {
  const at = stylesheet.indexOf(`\n${selector} {\n`);
  expect(at, `expected the stylesheet to hold \`${selector}\``).toBeGreaterThan(
    -1,
  );
  return stylesheet.slice(at, stylesheet.indexOf("\n}", at));
}
