//! The notices stream: its frames read into notices, whether one is worth
//! showing, and the holding of it against a real server.
//!
//! The holding is proved the way the health wait is, against an HTTP server the
//! test stands up — a stub over `fetch` would prove the loop against itself, and
//! what is worth proving is a stream that arrives in pieces, drops, and refuses a
//! key that has since been reset.

import { createServer, type ServerResponse, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import { Frames, hold, type Notice, notice, opens, pause, showing } from "../src/notices.js";

/// A Set's notice, as the server writes one.
const ASKED: Notice = {
  path: "/conversations/7/timeline#event-12",
  title: "Which database should the cache use?",
  project: "verkstead",
};

/// The frame the server sends it in.
const FRAME = `event: notice\ndata: ${JSON.stringify(ASKED)}\n\n`;

describe("reading a notice", () => {
  it("reads the three fields", () => {
    expect(notice(JSON.stringify(ASKED))).toEqual(ASKED);
  });

  /// A device asking to join is about no repository at all.
  it("reads one with no project", () => {
    expect(notice('{"path":"/","title":"A device asks to join"}')).toEqual({
      path: "/",
      title: "A device asks to join",
    });
  });

  it("refuses what is not one", () => {
    expect(notice("not json")).toBeUndefined();
    expect(notice("null")).toBeUndefined();
    expect(notice('{"title":"no path"}')).toBeUndefined();
    expect(notice('{"path":"/","title":3}')).toBeUndefined();
    expect(notice('{"path":"/","title":"t","project":4}')).toBeUndefined();
  });
});

describe("the stream's frames", () => {
  it("reads a whole frame", () => {
    expect(new Frames().push(FRAME)).toEqual([ASKED]);
  });

  it("reads several frames out of one chunk", () => {
    expect(new Frames().push(FRAME + FRAME)).toEqual([ASKED, ASKED]);
  });

  /// Chunks are wherever the network cut them, so every split of the frame has
  /// to come to the same one notice.
  it("reads a frame cut anywhere", () => {
    for (let at = 1; at < FRAME.length; at++) {
      const frames = new Frames();
      const read = [...frames.push(FRAME.slice(0, at)), ...frames.push(FRAME.slice(at))];
      expect(read, `cut at ${at}`).toEqual([ASKED]);
    }
  });

  it("reads CRLF and lone CR line endings", () => {
    expect(new Frames().push(FRAME.replaceAll("\n", "\r\n"))).toEqual([ASKED]);
  });

  /// A lone CR at the end of a chunk could be half a CRLF, so a frame ended by
  /// one is read once the next chunk says it was not.
  it("reads a lone CR line ending once the next chunk arrives", () => {
    const frames = new Frames();
    expect(frames.push(FRAME.replaceAll("\n", "\r"))).toEqual([]);
    expect(frames.push(": keep-alive\r")).toEqual([ASKED]);
  });

  /// A CRLF cut between its two halves is one line ending, not a blank line.
  it("holds a CR back until it knows whether a LF follows", () => {
    const crlf = FRAME.replaceAll("\n", "\r\n");
    const at = crlf.indexOf("\r\n") + 1;
    const frames = new Frames();
    expect([...frames.push(crlf.slice(0, at)), ...frames.push(crlf.slice(at))]).toEqual([ASKED]);
  });

  it("skips keep-alive comments", () => {
    expect(new Frames().push(`:\n\n: keep-alive\n\n${FRAME}`)).toEqual([ASKED]);
  });

  it("joins a notice split over several data lines", () => {
    const split = `event: notice\ndata: {"path":"/",\ndata: "title":"two lines"}\n\n`;
    expect(new Frames().push(split)).toEqual([{ path: "/", title: "two lines" }]);
  });

  it("skips frames that are not notices", () => {
    expect(new Frames().push(`event: other\ndata: ${JSON.stringify(ASKED)}\n\n`)).toEqual([]);
    expect(new Frames().push(`data: ${JSON.stringify(ASKED)}\n\n`)).toEqual([]);
    expect(new Frames().push("event: notice\ndata: broken\n\n")).toEqual([]);
    expect(new Frames().push("event: notice\n\n")).toEqual([]);
  });

  it("keeps an unfinished frame for the chunk after", () => {
    const frames = new Frames();
    expect(frames.push("event: notice\n")).toEqual([]);
    expect(frames.push(`data: ${JSON.stringify(ASKED)}\n`)).toEqual([]);
    expect(frames.push("\n")).toEqual([ASKED]);
  });
});

describe("whether a notice is shown", () => {
  const FRONT = { visible: true, minimized: false, focused: true };

  it("withholds one while the window is in front", () => {
    expect(showing(FRONT, true)).toEqual({ show: false, why: "the window is in front" });
  });

  it("shows one while the window is hidden to the tray", () => {
    expect(showing({ visible: false, minimized: false, focused: false }, true)).toEqual({ show: true });
  });

  it("shows one while the window is minimised", () => {
    expect(showing({ visible: true, minimized: true, focused: false }, true)).toEqual({ show: true });
  });

  it("shows one while the window is behind another app", () => {
    expect(showing({ ...FRONT, focused: false }, true)).toEqual({ show: true });
  });

  /// A login start's window was never shown, and may still say it has focus.
  it("shows one while the window has never been shown", () => {
    expect(showing({ visible: false, minimized: false, focused: true }, true)).toEqual({ show: true });
  });

  it("withholds every one while notifications are off", () => {
    expect(showing({ visible: false, minimized: false, focused: false }, false)).toEqual({
      show: false,
      why: "notifications are switched off",
    });
  });
});

describe("where a click opens", () => {
  /// The four kinds of path the server sends, each landing where a tap on the
  /// phone lands.
  it("is the notice's own path", () => {
    for (const path of [
      "/conversations/7/events/12",
      "/conversations/7",
      "/settings/remote",
      "/devices/laptop/conversations/7",
    ]) {
      expect(opens({ path, title: "news" })).toBe(path);
    }
  });

  /// A click is a navigation of the app's own window, which never leaves
  /// localhost: two slashes is another host to a browser, and a path with no
  /// slash in front is no path of this origin's.
  it("is the root where the path would leave the workbench", () => {
    for (const path of ["//elsewhere.example/x", "http://elsewhere.example/", "", "/\\elsewhere"]) {
      expect(opens({ path, title: "news" }), path).toBe("/");
    }
  });
});

describe("the wait between openings", () => {
  it("starts short and doubles", () => {
    expect([1, 2, 3].map((after) => pause(after))).toEqual([250, 500, 1_000]);
  });

  it("never passes the bound", () => {
    expect(pause(50)).toBe(5_000);
  });
});

/// Whatever the test's server is doing with each request that arrives.
type Answering = (cookie: string | undefined, response: ServerResponse) => void;

let standing_up: Server | undefined;
let holding: AbortController | undefined;

afterEach(async () => {
  holding?.abort();
  holding = undefined;
  if (standing_up) {
    standing_up.closeAllConnections();
    await new Promise<void>((shut) => standing_up!.close(() => shut()));
    standing_up = undefined;
  }
});

/// A server that answers every request with `answering`, on a port the machine
/// picked.
function standing(answering: Answering): Promise<string> {
  const server = createServer((request, response) => answering(request.headers.cookie, response));
  standing_up = server;
  return new Promise((up) => {
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address() as AddressInfo;
      up(`http://127.0.0.1:${port}/api/ui/push/notices`);
    });
  });
}

/// Begin the event stream on `response`.
function streaming(response: ServerResponse): void {
  response.writeHead(200, { "content-type": "text/event-stream" });
  response.flushHeaders();
}

/// Hold the stream at `url` with the key `secret` says, and collect what it
/// hears and says.
function held(url: string, secret: () => string | undefined) {
  const heard: Notice[] = [];
  const said: string[] = [];
  holding = new AbortController();
  const done = hold(
    {
      url,
      secret,
      heard: (arrived) => heard.push(arrived),
      say: (line) => said.push(line),
      waiting: { first: 5, most: 20 },
    },
    holding.signal,
  );
  return { heard, said, done };
}

/// Wait until `condition` holds, for up to two seconds.
async function until(condition: () => boolean): Promise<void> {
  const started = Date.now();
  while (!condition()) {
    if (Date.now() - started > 2_000) {
      throw new Error("never happened");
    }
    await new Promise((later) => setTimeout(later, 5));
  }
}

describe("holding the stream", () => {
  it("logs in with the key and hands on each notice", async () => {
    const url = await standing((cookie, response) => {
      expect(cookie).toBe("workbench_key=secret");
      streaming(response);
      response.write(": keep-alive\n\n");
      response.write(FRAME.slice(0, 20));
      setTimeout(() => response.write(FRAME.slice(20)), 10);
    });

    const { heard, said } = held(url, () => "secret");

    await until(() => heard.length === 1);
    expect(heard).toEqual([ASKED]);
    expect(said).toContain(`the notices stream is held at ${url}`);
  });

  /// **Reset key** from the phone: the old key is refused, the file is read
  /// again, and the stream is held under the new one.
  it("reads the key again after a 401", async () => {
    let onDisk = "old";
    const url = await standing((cookie, response) => {
      if (cookie !== "workbench_key=new") {
        onDisk = "new";
        response.writeHead(401).end();
        return;
      }
      streaming(response);
      response.write(FRAME);
    });

    const { heard, said } = held(url, () => onDisk);

    await until(() => heard.length === 1);
    expect(said[0]).toMatch(/refused the key/);
    expect(said).toContain(`the notices stream is held at ${url}`);
  });

  /// A stream already open is not refused by a reset, so the app notices the
  /// file changing and holds the stream again under the new key.
  it("opens the stream again under a key reset while it is held", async () => {
    let onDisk = "old";
    const cookies: (string | undefined)[] = [];
    const url = await standing((cookie, response) => {
      cookies.push(cookie);
      streaming(response);
      if (cookie === "workbench_key=old") {
        onDisk = "new";
        response.write(": keep-alive\n\n");
      } else {
        response.write(FRAME);
      }
    });

    const { heard, said } = held(url, () => onDisk);

    await until(() => heard.length === 1);
    expect(cookies).toEqual(["workbench_key=old", "workbench_key=new"]);
    expect(said).toContain(
      "the Workbench Key was reset, so the notices stream is opened again under the new one",
    );
  });

  it("opens the stream again after it ends", async () => {
    let opened = 0;
    const url = await standing((_, response) => {
      opened += 1;
      streaming(response);
      response.end(FRAME);
    });

    const { heard, said } = held(url, () => "secret");

    await until(() => opened >= 3);
    expect(heard.length).toBeGreaterThanOrEqual(2);
    expect(said).toContain("the notices stream ended, so it is opened again");
  });

  it("waits out a server that is not there", async () => {
    const url = await standing(() => {});
    standing_up!.close();
    standing_up = undefined;

    const { said, done } = held(url, () => "secret");

    await until(() => said.filter((line) => line.includes("dropped")).length >= 2);
    holding!.abort();
    await expect(done).resolves.toBeUndefined();
  });

  it("stops when it is told to", async () => {
    const url = await standing((_, response) => streaming(response));

    const { said, done } = held(url, () => "secret");

    await until(() => said.length === 1);
    holding!.abort();
    await expect(done).resolves.toBeUndefined();
  });
});
