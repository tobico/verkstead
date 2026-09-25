//! The rule that decides what the window keeps and what the browser gets.
//!
//! The list below is the one that matters: the workbench's own pages in every
//! spelling the loopback has, the links that lead off it, and the several ways
//! a page can ask for something that is not a page at all. A miss in the first
//! group is a workbench that leaves for the browser mid-click; a miss in the
//! second is an app that has become a browser with no address bar; a miss in
//! the third is a link on a page starting a program on somebody's machine.

import { describe, expect, it } from "vitest";

import { where } from "../src/elsewhere.js";
import { ORIGIN } from "../src/workbench.js";

describe("what the window keeps", () => {
  it("keeps the workbench itself", () => {
    expect(where(ORIGIN, ORIGIN)).toBe("window");
    expect(where(`${ORIGIN}/`, ORIGIN)).toBe("window");
  });

  it("keeps every page of it, and the login link", () => {
    for (const path of ["/tasks", "/settings/remote-access", "/?key=abc123", "/api/v1/health"]) {
      expect(where(`${ORIGIN}${path}`, ORIGIN), path).toBe("window");
    }
  });

  /// One host, three spellings, and the workbench's own links need not agree
  /// with the address the app happened to load.
  it("keeps the loopback however it is spelled", () => {
    for (const host of ["127.0.0.1", "localhost", "LOCALHOST", "127.1", "[::1]"]) {
      expect(where(`http://${host}:8422/tasks`, ORIGIN), host).toBe("window");
    }
  });
});

describe("what goes to the browser", () => {
  it("hands over the links that lead off the workbench", () => {
    for (const link of [
      "https://github.com/verkstead/verkstead/pull/1",
      "https://gist.github.com/anon/0123456789abcdef",
      "http://example.com/a-page",
      "https://example.com:8422/same-port-elsewhere",
    ]) {
      expect(where(link, ORIGIN), link).toBe("browser");
    }
  });

  /// A different port on this machine is a different program. There is one
  /// Verkstead here and it is on the one address.
  it("hands over another program on this same machine", () => {
    expect(where("http://127.0.0.1:5173/", ORIGIN)).toBe("browser");
    expect(where("http://localhost/", ORIGIN)).toBe("browser");
  });

  /// The same server over TLS is not the server this app started, which serves
  /// the loopback in the clear.
  it("hands over the address on another scheme", () => {
    expect(where("https://127.0.0.1:8422/", ORIGIN)).toBe("browser");
  });
});

describe("what happens nowhere at all", () => {
  /// Everything here would otherwise be the platform's opener asked to start a
  /// program, over a link that a page put on itself.
  it("refuses every scheme a browser is not for", () => {
    for (const target of [
      "mailto:someone@example.com",
      "file:///etc/passwd",
      "javascript:alert(1)",
      "data:text/html,<h1>hello</h1>",
      "ms-msdt:/id",
      "smb://server/share",
      "vscode://file/etc/passwd",
    ]) {
      expect(where(target, ORIGIN), target).toBe("nowhere");
    }
  });

  it("refuses what is not a URL at all", () => {
    for (const target of ["", "not a url", "://", "/tasks"]) {
      expect(where(target, ORIGIN), JSON.stringify(target)).toBe("nowhere");
    }
  });
});
