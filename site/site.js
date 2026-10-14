// The page works without this file: the tabs are links to the sections, the
// download button links to the table, and every card's screenshot shows in
// order. What it adds is the pressed tab following the reader, the download
// button naming the reader's own platform, and the cards becoming carousels.

const RELEASE = "https://github.com/tobico/verkstead/releases/latest/download/";

// The asset for this browser, or null where there is no one right answer: an
// unknown platform, an ARM Linux, and any phone or tablet.
function downloadFor(platform, userAgent, touchPoints) {
  const ua = userAgent || "";
  const p = (platform || "").toLowerCase();
  if (/android|iphone|ipad|ipod/i.test(ua)) return null;
  const mac = p ? p === "macos" || p.startsWith("mac") : /macintosh|mac os x/i.test(ua);
  if (mac) {
    // An iPad asks for the desktop site as a Mac, and only its touch gives it away.
    if (touchPoints > 1) return null;
    return { label: "macOS", asset: "Verkstead-universal.dmg" };
  }
  const windows = p ? p.startsWith("win") : /windows/i.test(ua);
  if (windows) return { label: "Windows", asset: "Verkstead-x86_64.msi" };
  const linux = p ? p === "linux" || p.startsWith("linux") : /linux/i.test(ua);
  if (linux) {
    if (/aarch64|arm/i.test(ua) || /aarch64|arm/.test(p)) return null;
    return { label: "Linux", asset: "Verkstead-x86_64.AppImage" };
  }
  return null;
}

function labelDownloads() {
  const found = downloadFor(
    navigator.userAgentData?.platform,
    navigator.userAgent,
    navigator.maxTouchPoints || 0,
  );
  if (!found) return;
  for (const link of document.querySelectorAll("[data-download]")) {
    link.href = RELEASE + found.asset;
    const label = link.querySelector("[data-download-label]");
    if (label) label.textContent = `Download for ${found.label}`;
  }
}

function trackSections() {
  const nav = document.querySelector(".tabs");
  const indicator = nav.querySelector(".tabs-indicator");
  const tabs = [...nav.querySelectorAll(".tab")];
  const sections = tabs.map((tab) => document.querySelector(tab.getAttribute("href")));
  const reduced = matchMedia("(prefers-reduced-motion: reduce)");
  let pressed = null;

  function place() {
    if (!pressed) {
      indicator.style.width = "0";
      return;
    }
    indicator.style.width = `${pressed.offsetWidth}px`;
    indicator.style.transform = `translateX(${pressed.offsetLeft}px)`;
  }

  function press(tab) {
    if (tab === pressed) return;
    if (pressed) pressed.removeAttribute("aria-current");
    pressed = tab;
    if (tab) tab.setAttribute("aria-current", "true");
    place();
  }

  // The section crossing a line a third of the way down the viewport is the one
  // being read; above the first and below the last, none is.
  let clicked = null;
  let releasing;
  function follow() {
    if (clicked) return;
    const line = innerHeight / 3;
    let current = null;
    sections.forEach((section, i) => {
      const box = section.getBoundingClientRect();
      if (box.top <= line && box.bottom > line) current = tabs[i];
    });
    press(current);
  }

  tabs.forEach((tab, i) => {
    tab.addEventListener("click", (event) => {
      event.preventDefault();
      press(tab);
      // Hold the clicked tab while a smooth scroll passes the others.
      clicked = tab;
      sections[i].scrollIntoView({ behavior: reduced.matches ? "auto" : "smooth" });
      history.replaceState(null, "", tab.getAttribute("href"));
      clearTimeout(releasing);
      const release = () => {
        clicked = null;
        follow();
      };
      if ("onscrollend" in window && !reduced.matches) {
        addEventListener("scrollend", release, { once: true });
        releasing = setTimeout(release, 1500);
      } else {
        releasing = setTimeout(release, 50);
      }
    });
  });

  addEventListener("scroll", follow, { passive: true });
  addEventListener("resize", () => {
    place();
    follow();
  });
  follow();
}

// Each carousel presses one card, and the stylesheet shows that card's panel
// and hides the rest. Which shape it takes, a column beside the panel or an
// accordion, is the stylesheet's too, so the pressed card survives a resize
// across the breakpoint untouched.
function carousels() {
  for (const carousel of document.querySelectorAll(".carousel")) {
    const cards = [...carousel.querySelectorAll(".card")];
    const show = (pressed) => {
      for (const card of cards) card.setAttribute("aria-pressed", String(card === pressed));
    };
    for (const card of cards) card.addEventListener("click", () => show(card));
    show(cards[0]);
  }
}

if (typeof document !== "undefined") {
  carousels();
  labelDownloads();
  trackSections();
}

if (typeof module !== "undefined") module.exports = { downloadFor };
