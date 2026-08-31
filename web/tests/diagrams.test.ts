//! Drawing the Diagrams on a Set's page.
//!
//! The renderer itself is a stand-in throughout: what is under test is which
//! blocks are drawn, what happens to one that will not draw, and that a page
//! whose colours change is drawn again — none of which needs mermaid to be here
//! to be asked.

import { fireEvent, waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";

import { drawDiagrams } from "../src/set/diagrams";
// Two sheets as text. The palette, because what the theme is spent out of is
// what the page defines; and the markdown, because the two rules asserted at the
// bottom are about a drawn diagram — an SVG mermaid wrote and no component
// renders, so there is nothing to query them off.
import base from "../src/styles/base.css?raw";
import markdown from "../src/styles/markdown.css?raw";

/// A page carrying the source blocks the markdown renderer leaves behind —
/// escaped, exactly as the server wrote them.
function page(...sources: string[]) {
  document.body.innerHTML = sources
    .map((source) => `<pre class="mermaid">${source}</pre>`)
    .join("");
}

/// The colour scheme, as something a test can flip. jsdom answers the media
/// query but has no way to change its mind, and following a change of scheme is
/// half of what this module does.
function scheme() {
  const listeners = new Set<() => void>();
  const query = {
    matches: false,
    addEventListener: (_: string, listen: () => void) => listeners.add(listen),
    removeEventListener: (_: string, listen: () => void) =>
      listeners.delete(listen),
  };

  vi.stubGlobal("matchMedia", () => query);

  return {
    flip() {
      query.matches = !query.matches;
      for (const listen of [...listeners]) listen();
    },
    watched: () => listeners.size,
  };
}

/// The page's palette, as something a test can read back out of what mermaid was
/// told. jsdom resolves no custom property, so `getComputedStyle` is stood in for:
/// every `--name` asked for answers with a marker naming itself, which is what
/// lets an assertion say *which* variable a theme colour was spent from.
///
/// It also records the asking, so the second half of the pairing can be checked:
/// every property the renderer reaches for has to be one the stylesheet defines.
function palette() {
  const asked = new Set<string>();

  vi.stubGlobal("getComputedStyle", () => ({
    getPropertyValue: (name: string) => {
      asked.add(name);
      return ` spent(${name}) `;
    },
    fontFamily: "the page's own type",
  }));

  return {
    asked: () => [...asked],
    /// What a variable comes out as once the renderer has trimmed it.
    spent: (name: string) => `spent(${name})`,
  };
}

/// A stand-in for the renderer: what it was told about the page, what it was
/// asked to draw, and whatever `drawing` makes of each source — `null` for the
/// two ways mermaid does not draw, which it never tells apart.
function renderer(drawing: (text: string) => string | null) {
  const configured: Array<Record<string, unknown>> = [];
  const asked: Array<{ id: string; text: string }> = [];

  const bundle = () =>
    Promise.resolve({
      initialize(config: Record<string, unknown>) {
        configured.push(config);
      },
      render(id: string, text: string) {
        asked.push({ id, text });
        const svg = drawing(text);
        return svg === null
          ? Promise.reject(new Error("will not draw"))
          : Promise.resolve({ svg });
      },
    });

  return { bundle, configured, asked };
}

/// A drawing of whatever it was handed, so a test can tell one apart from
/// another without a renderer in the room.
function drawn(text: string): string {
  return `<svg data-source="${text.trim()}"></svg>`;
}

/// The same drawing, carrying what mermaid writes onto a real one: the width as
/// `100%`, which is what it says whenever a diagram may fill the room it is given,
/// the height as the length it measured, the shape in the `viewBox`, and the
/// measurement again as the inline ceiling it puts on itself.
function sized(text: string): string {
  const measured = 'width="100%" height="200" viewBox="0 0 400 200"';
  const ceiling = 'style="max-width: 400px;"';

  return `<svg ${measured} ${ceiling} data-source="${text.trim()}"></svg>`;
}

/// The declarations of the block `selector` opens, which is what a rule about a
/// drawn diagram has to be read out of: the SVG is mermaid's and the chrome around
/// it is `diagrams.ts`'s, so there is no component to query either off.
function block(selector: string): string {
  const opened = markdown.indexOf(`${selector} {`);
  expect(opened, `the stylesheet should have a \`${selector}\` rule`).not.toBe(-1);

  return markdown.slice(opened, markdown.indexOf("}", opened));
}

afterEach(() => {
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

describe("the Diagrams on a page", () => {
  it("draws every source block, in place and unescaped", async () => {
    scheme();
    const { bundle, asked } = renderer(drawn);
    page("graph LR;\n  client--&gt;api;\n", "graph TD;\n  a--&gt;b;\n");

    drawDiagrams({ bundle });

    // The figures rather than the asking: a block is replaced once the drawing
    // of it comes back, which is a turn after the renderer was handed it.
    //
    // A `div` in place of the `pre`, because the stylesheet washes and boxes
    // every `pre` in rendered markdown as the code it usually is.
    await waitFor(() =>
      expect(document.querySelectorAll("div.diagram")).toHaveLength(2),
    );

    // `textContent` is what takes the escaping back off, so the renderer is
    // handed what the agent wrote rather than what the page holds.
    expect(asked[0]!.text).toBe("graph LR;\n  client-->api;\n");
    expect(asked[1]!.text).toBe("graph TD;\n  a-->b;\n");

    const figures = document.querySelectorAll("div.diagram");
    expect(figures[0]!.innerHTML).toContain('data-source="graph LR;');
    expect(document.querySelectorAll("pre.mermaid")).toHaveLength(0);
  });

  it("names each drawing on the page differently", async () => {
    scheme();
    const { bundle, asked } = renderer(drawn);
    page("graph LR;\n  a--&gt;b;\n", "graph LR;\n  a--&gt;b;\n");

    drawDiagrams({ bundle });
    await waitFor(() => expect(asked).toHaveLength(2));

    // Mermaid stamps the id it is handed all the way through the SVG it gives
    // back, so two drawings on one page may not share one.
    expect(asked[0]!.id).not.toBe(asked[1]!.id);
  });

  it("leaves a diagram it cannot draw as the source the agent wrote", async () => {
    scheme();
    const { bundle, asked } = renderer((text) =>
      text.includes("not a diagram") ? null : drawn(text),
    );
    page("not a diagram at all\n", "graph LR;\n  a--&gt;b;\n");

    drawDiagrams({ bundle });
    await waitFor(() => expect(asked).toHaveLength(2));
    await waitFor(() => expect(document.querySelector("div.diagram")).toBeTruthy());

    const left = document.querySelectorAll("pre.mermaid");
    expect(left).toHaveLength(1);
    expect(left[0]!.textContent).toBe("not a diagram at all\n");

    // The fallback is silent: a human reads the source the agent wrote rather
    // than mermaid's complaint about it.
    for (const complaint of ["Syntax error", "error in text", "mermaid-error"]) {
      expect(document.body.innerHTML).not.toContain(complaint);
    }
  });

  it("leaves every block standing when the renderer never arrives", async () => {
    scheme();
    page("graph LR;\n  a--&gt;b;\n");

    drawDiagrams({ bundle: () => Promise.reject(new Error("never landed")) });

    // Nothing to wait for, so the assertion is that nothing happens: a page of
    // readable source blocks is the right page to be left with.
    await Promise.resolve();
    expect(document.querySelectorAll("pre.mermaid")).toHaveLength(1);
    expect(document.querySelector("div.diagram")).toBeNull();
  });

  it("tells the renderer that nothing in a diagram may run or be complained about", async () => {
    scheme();
    const { bundle, configured } = renderer(drawn);
    page("graph LR;\n  a--&gt;b;\n");

    drawDiagrams({ bundle });
    await waitFor(() => expect(configured).toHaveLength(1));

    // Every diagram here was written by an agent, so the labels are sanitized
    // and the click handlers and inline styles a diagram can ask for refused;
    // the deciding of what is drawn is ours; and a diagram mermaid cannot parse
    // must not be drawn as its own error graphic, because the source block is
    // the error state.
    expect(configured[0]).toMatchObject({
      startOnLoad: false,
      securityLevel: "strict",
      suppressErrorRendering: true,
    });
  });

  it("draws what drew again when the colour scheme flips", async () => {
    const colours = scheme();
    const { bundle, asked } = renderer((text) =>
      text.includes("not a diagram") ? null : drawn(text),
    );
    page("graph LR;\n  a--&gt;b;\n", "not a diagram at all\n");

    drawDiagrams({ bundle });
    await waitFor(() => expect(asked).toHaveLength(2));

    colours.flip();

    // The one that drew, and only that one: the other is still a source block,
    // and the stylesheet themes those along with the rest of the page.
    await waitFor(() => expect(asked).toHaveLength(3));
    expect(asked[2]!.text).toBe("graph LR;\n  a-->b;\n");
    expect(document.querySelectorAll("div.diagram")).toHaveLength(1);
  });

  it("stops watching the colour scheme once the page has gone", async () => {
    const colours = scheme();
    const { bundle, asked } = renderer(drawn);
    page("graph LR;\n  a--&gt;b;\n");

    const stop = drawDiagrams({ bundle });
    await waitFor(() => expect(asked).toHaveLength(1));

    stop();
    expect(colours.watched()).toBe(0);

    colours.flip();
    await Promise.resolve();
    expect(asked).toHaveLength(1);
  });
});

describe("the colours a Diagram is drawn in", () => {
  /// What mermaid was told before it drew, over a stood-in palette.
  async function told() {
    scheme();
    const ink = palette();
    const { bundle, configured } = renderer(drawn);
    page("graph LR;\n  a--&gt;b;\n");

    drawDiagrams({ bundle });
    await waitFor(() => expect(configured).toHaveLength(1));

    return { config: configured[0]!, ink };
  }

  it("is mermaid's one theme that brings no palette of its own", async () => {
    const { config } = await told();

    // `base` is the only mermaid theme that is all overrides. Every other one
    // arrives with a palette, and a second palette on the page is the thing this
    // is here to avoid.
    expect(config.theme).toBe("base");
  });

  it("is the page's, read off the document rather than written out again", async () => {
    const { config, ink } = await told();
    const variables = config.themeVariables as Record<string, unknown>;

    // Each of these is spent from a named variable rather than from a literal, so
    // a diagram cannot drift from the page it sits on — including in the dark
    // scheme, which the stylesheet is the only thing that knows about.
    expect(variables.background).toBe(ink.spent("--card"));
    expect(variables.primaryColor).toBe(ink.spent("--code-wash"));
    expect(variables.primaryTextColor).toBe(ink.spent("--ink"));
    expect(variables.lineColor).toBe(ink.spent("--ink-soft"));
    expect(variables.clusterBkg).toBe(ink.spent("--hunk"));
    expect(variables.clusterBorder).toBe(ink.spent("--edge"));
    expect(variables.noteBkgColor).toBe(ink.spent("--marked"));

    // The type too, so the font stack is written down in one place.
    expect(variables.fontFamily).toBe("the page's own type");
  });

  it("marks a tagged node in the Diff's own colours", async () => {
    const { config, ink } = await told();
    const css = config.themeCSS as string;

    // The three classes an agent puts on a node, and the pair of variables each
    // spends: the wash behind the node and the saturated ink around it, which is
    // the Diff's own pattern for a line it added or removed. `modified` is the one
    // the Diff has no colour for — it marks lines, and a changed line there is an
    // added one beside a removed one — so it takes the page's "look at this" wash,
    // outlined in the accent.
    for (const [tag, wash, edge] of [
      ["new", "--added-wash", "--added"],
      ["modified", "--marked", "--accent"],
      ["removed", "--removed-wash", "--removed"],
    ] as const) {
      const mark = css
        .split("\n")
        .find((rule) => rule.includes(`.node.${tag} `));

      expect(mark, `a \`${tag}\` node should be marked`).toBeTruthy();
      expect(mark).toContain(`fill: ${ink.spent(wash)}`);
      expect(mark).toContain(`stroke: ${ink.spent(edge)}`);
    }

    // Every selector is qualified by the class it marks, so a node nobody tagged
    // is left exactly as the theme drew it.
    expect(css).not.toMatch(/(^|[\s,]) *\.node +[a-z]+ *\{/);
  });

  it("spends nothing the stylesheet does not define", async () => {
    const { ink } = await told();

    // The other half of the pairing above: a variable the renderer reaches for and
    // the stylesheet has never heard of resolves to nothing, and mermaid then
    // derives a colour of its own by rotating a hue — which is how a colour that
    // is on no palette at all ends up on the page.
    for (const property of ink.asked()) {
      expect(
        base.includes(`${property}:`),
        `${property} should be a variable the palette defines`,
      ).toBe(true);
    }
  });

  it("has a value for every mark in each of the page's two schemes", async () => {
    const { ink } = await told();

    // The marks are the one part of the theme handed over as CSS, and a scheme
    // that left one of them undefined would draw that mark as nothing at all.
    for (const property of ink.asked()) {
      expect(
        base.split(`${property}:`).length - 1,
        `${property} should be defined in both schemes`,
      ).toBeGreaterThanOrEqual(2);
    }
  });
});

describe("a drawn Diagram on the page", () => {
  it("fits the width it is given", () => {
    const svg = block(".markdown .diagram svg");

    // At a glance means the whole shape at once, so a diagram too wide for a phone
    // scales down to fit rather than scrolling sideways inside a box — and never
    // widens the page, which is the failure a narrow viewport shows first.
    expect(svg).toContain("max-width: 100%");
    // The height follows the width, which it only does if the height mermaid wrote
    // onto the SVG is overridden.
    expect(svg).toContain("height: auto");
  });

  it("holds still for anyone who asked it to", () => {
    // A mermaid diagram can ask for animated edges, and the animation arrives
    // inside the SVG in a stylesheet of mermaid's own — so this is the one place
    // in the file where turning something off has to out-rank an author.
    const reduced = markdown.slice(
      markdown.indexOf("@media (prefers-reduced-motion: reduce)"),
    );

    expect(reduced).toContain(".diagram");
    expect(reduced).toContain("animation: none !important");
  });
});

describe("expanding a Diagram", () => {
  /// A page of one drawing, drawn and then expanded: what the lightbox is, and
  /// the button it was opened from.
  async function expanded(drawing: (text: string) => string | null = sized) {
    scheme();
    const { bundle, asked } = renderer(drawing);
    page("graph LR;\n  a--&gt;b;\n");

    const stop = drawDiagrams({ bundle });
    await waitFor(() => expect(asked).toHaveLength(1));

    const expand = document.querySelector<HTMLButtonElement>(
      "div.diagram button",
    );
    expect(expand).toBeTruthy();

    fireEvent.click(expand!);

    const lightbox = document.querySelector<HTMLDialogElement>(
      "dialog.diagram-lightbox",
    );
    expect(lightbox).toBeTruthy();

    return { lightbox: lightbox!, expand: expand!, stop };
  }

  it("hangs a button on every drawing that landed, and on nothing else", async () => {
    scheme();
    const { bundle, asked } = renderer((text) =>
      text.includes("not a diagram") ? null : drawn(text),
    );
    page("not a diagram at all\n", "graph LR;\n  a--&gt;b;\n");

    drawDiagrams({ bundle });
    await waitFor(() => expect(asked).toHaveLength(2));

    // One button for the one drawing, inside the figure and not in the card
    // around it. The block that would not draw is the source the agent wrote and
    // nothing else — there is nothing there to expand.
    const buttons = document.querySelectorAll("button");
    expect(buttons).toHaveLength(1);
    expect(buttons[0]!.closest("div.diagram")).toBeTruthy();
    expect(document.querySelector("pre.mermaid button")).toBeNull();

    // The icon says nothing when it is read aloud, so the label is the whole of
    // what the button is called.
    expect(buttons[0]!.getAttribute("aria-label")).toBe("Expand");
    expect(buttons[0]!.querySelector("svg")!.getAttribute("aria-hidden")).toBe(
      "true",
    );
  });

  it("draws a copy of it over the page, and leaves the page whole", async () => {
    const { lightbox } = await expanded();

    expect(lightbox.open).toBe(true);

    // The same drawing, in the box the collapse button is positioned against.
    const copy = lightbox.querySelector("div.diagram-drawing > svg");
    expect(copy!.getAttribute("data-source")).toBe("graph LR;\n  a-->b;");

    // A copy: the card behind the blanker still holds the drawing it was opened
    // from, which is what the letterboxed bands show through to.
    expect(document.querySelector("div.diagram svg")).toBeTruthy();

    const collapse = lightbox.querySelector("button");
    expect(collapse!.getAttribute("aria-label")).toBe("Collapse");
  });

  it("letterboxes the copy, and stops at twice what mermaid measured", async () => {
    const { lightbox } = await expanded();

    const box = lightbox.querySelector<HTMLElement>("div.diagram-drawing")!;

    // The width worked back out of the height and the shape, mermaid having
    // written the width itself as `100%`; and the shape, which is what keeps a
    // tall diagram's bottom on the screen.
    expect(box.style.getPropertyValue("--diagram-size")).toBe("400px");
    expect(box.style.getPropertyValue("--diagram-ratio")).toBe("2");

    // And what the stylesheet spends them on: the smallest of twice the
    // measurement, the window's width, and the width this shape may have in the
    // window's height.
    const rule = block(".diagram-lightbox > .diagram-drawing");
    expect(rule).toContain("calc(var(--diagram-size, 100%) * 2)");
    expect(rule).toContain("var(--diagram-ratio, 1)");

    // The one thing the copy may not keep: mermaid writes the size it measured
    // onto the drawing as an inline ceiling, and the lightbox is the one place a
    // diagram is drawn deliberately bigger than that.
    const copy = box.querySelector<SVGElement>("svg")!;
    expect(copy.style.maxWidth).toBe("none");
  });

  it("caps the drawing in its card at the width mermaid measured", async () => {
    scheme();
    const { bundle, asked } = renderer(sized);
    page("graph LR;\n  a--&gt;b;\n");

    drawDiagrams({ bundle });
    await waitFor(() => expect(asked).toHaveLength(1));

    // The box the button is positioned against is the drawing's own width, so the
    // corner it floats in is the drawing's corner rather than the figure's.
    const box = document.querySelector<HTMLElement>("div.diagram-drawing")!;
    expect(box.style.getPropertyValue("--diagram-size")).toBe("400px");
    expect(block(".markdown .diagram > .diagram-drawing")).toContain(
      "max-width: var(--diagram-size, 100%)",
    );
  });

  it("closes on the collapse button, on the drawing, and on the blanker", async () => {
    for (const pressed of [
      (lightbox: HTMLDialogElement) => lightbox.querySelector("button")!,
      (lightbox: HTMLDialogElement) => lightbox.querySelector("svg")!,
      (lightbox: HTMLDialogElement) => lightbox,
    ]) {
      const { lightbox } = await expanded();

      fireEvent.click(pressed(lightbox));

      // Closed and gone: a dialog merely closed would still be in the document
      // for the next press to find.
      expect(lightbox.open).toBe(false);
      expect(document.querySelector("dialog.diagram-lightbox")).toBeNull();

      document.body.innerHTML = "";
    }
  });

  it("closes on Escape, which is the platform's", async () => {
    const { lightbox } = await expanded();

    fireEvent.keyDown(document, { key: "Escape" });

    expect(lightbox.open).toBe(false);
    expect(document.querySelector("dialog.diagram-lightbox")).toBeNull();
  });

  it("opens again after it has been closed", async () => {
    const { lightbox, expand } = await expanded();
    fireEvent.click(lightbox);

    fireEvent.click(expand);

    // The button is still the page's own and still opens: a redraw puts the same
    // one back, and a close takes only the dialog.
    expect(document.querySelector("dialog.diagram-lightbox")).toBeTruthy();
  });

  it("follows the page's colours while it is open", async () => {
    const colours = scheme();

    // The same source drawn twice over comes back differently, which is what a
    // change of scheme is: the drawing is mermaid's idea of the page's colours at
    // the moment it was asked.
    let pass = 0;
    const { bundle, asked } = renderer(() => {
      pass += 1;
      return `<svg data-pass="${pass}"></svg>`;
    });
    page("graph LR;\n  a--&gt;b;\n");

    drawDiagrams({ bundle });
    await waitFor(() => expect(asked).toHaveLength(1));

    fireEvent.click(document.querySelector("div.diagram button")!);

    const copy = () =>
      document
        .querySelector("dialog.diagram-lightbox svg")!
        .getAttribute("data-pass");

    expect(copy()).toBe("1");

    colours.flip();
    await waitFor(() => expect(asked).toHaveLength(2));

    // Both of them: an expanded diagram in last scheme's colours is the one thing
    // the page behind it would not be.
    await waitFor(() => expect(copy()).toBe("2"));
    expect(
      document.querySelector("div.diagram svg")!.getAttribute("data-pass"),
    ).toBe("2");
  });

  it("takes the lightbox with it when the page goes", async () => {
    const { lightbox, stop } = await expanded();

    stop();

    // Closed rather than removed, which is how the focus finds its way back out
    // of the top layer.
    expect(lightbox.open).toBe(false);
    expect(document.querySelector("dialog.diagram-lightbox")).toBeNull();
  });
});

describe("the lightbox a Diagram is expanded into", () => {
  it("dims the page the way the app's other dialog does", () => {
    // The same strength as `.modal::backdrop`, and for the same reason: what the
    // drawing does not cover is the page, still there under it.
    expect(block(".diagram-lightbox::backdrop")).toContain("rgb(0 0 0 / 45%)");
  });

  it("floats the button until the diagram is looked at, where anything hovers", () => {
    // Nothing about the button at rest says it is hidden: on a device that cannot
    // hover, hidden until hover is hidden for good.
    // The transition is what it appears through, not what it is at rest.
    expect(block(".diagram-button")).not.toContain("opacity:");

    const hovers = markdown.slice(markdown.indexOf("@media (hover: hover)"));

    // Out of the way until the diagram is hovered — in the card and in the
    // lightbox alike — or until a keyboard reaches the button, hovering being the
    // one way to it a keyboard has not got.
    expect(hovers).toContain("opacity: 0");
    expect(hovers).toContain(".diagram:hover .diagram-button");
    expect(hovers).toContain(".diagram-lightbox:hover .diagram-button");
    expect(hovers).toContain(".diagram-button:focus-visible");
  });
});
