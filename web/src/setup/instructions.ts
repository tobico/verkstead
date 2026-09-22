//! What to run on this machine to get what a session needs — one set of
//! instructions per operating system, and one instruction per row.
//!
//! **The commands are exact, or there is no command.** A row on a machine whose
//! package manager carries the program says the line to paste; a row on one that
//! does not says where the vendor's own installer is and nothing else. What is
//! never written here is a command that would install a *different* program
//! under the right name — see [`GROK_UNIX`], which is the whole of why Grok
//! Build is a link on all nine tabs.
//!
//! **Every instruction says where the binary has to land**, because installing
//! one is only half of it: a session resolves its programs on the `PATH` the
//! server itself was started with, composed for a session — see
//! `crates/server/src/sandbox.rs`'s `composed`. Which directories those are is a
//! fact about the machine rather than about the tab, so the wizard draws the
//! server's own list above the rows and nothing here says it: see
//! `OnboardingView.path`, which is where that list comes from. And that the
//! `PATH` is read once, at startup, is the one sentence every tab would carry
//! word for word, so it is drawn above the rows too — see `Dependencies.tsx`'s
//! `RESTART`.
//!
//! **A row may lead with one install and keep another under it.** Claude Code is
//! the row that does, on every tab but the Mac's: the vendor's own installer
//! first, because a distribution's package can be too old to connect at all —
//! Ubuntu's under WSL was — and the packaged one under it for the machine that
//! would rather have that. See [`Instruction`]'s `alternative`. The Mac leads
//! with the same installer and keeps nothing under it: what used to be there
//! was Homebrew's cask, which existed for the Dock's `PATH` alone — see
//! [`CLAUDE_ON_A_MAC`]. The Apple-silicon Mac's `git` row is the other one that
//! keeps a second: Apple's own dialog leads it on both Mac tabs, because the
//! `/usr/bin/git` every Mac has is a stub without the command line tools, and
//! `brew install git` is under it on the tab that has a `brew`.
//!
//! **Nine tabs and not one**, because the detection is a guess. It comes off
//! `/etc/os-release`'s `ID` and then `ID_LIKE` on a Linux, and off
//! `hw.optional.arm64` on a Mac, so a derivative names its parent and something
//! nobody has heard of names nothing: the detected tab opens and the other
//! eight stay a press away, for the machine the read was wrong about.
//!
//! Nothing here is on the wire. What the server says is what this machine *is*
//! and what it is missing; what to do about it is the same nine answers on
//! every Verkstead, so they are the viewer's own — see
//! `crates/render/src/onboarding.rs`, which carries the rows and the distro and
//! no prose at all.

import type { Dependency, Distro } from "../api/types";

/// One row's instruction, on one operating system.
///
/// A command, a link, or a note by itself — the sandbox row on macOS is nothing
/// to install, and *other Linux* has the generic list of what is needed where
/// the five named distributions have a line to paste.
///
/// The Windows sandbox row is neither of those: it is an account rather than a
/// program, and the wizard makes one where it is ticked. What its command field
/// carries is the same thing by hand, which is the line for whoever reaches
/// this screen because Windows would not raise the dialog — and its note says
/// which terminal that line has to be pasted into.
export type Instruction = {
  /// What to run, exactly, where this OS carries the program.
  command?: string;

  /// Or the vendor's own install page, where it does not.
  link?: string;

  /// And what the command does not say for itself: where the program has to
  /// land, what else it wants installed first, and what a machine has to be
  /// told before the binary is one a session can open.
  note?: string;

  /// Or the other way to get the same program, drawn under the first with a
  /// note of its own.
  ///
  /// Two instructions rather than one whose note names a second command: the
  /// second is a line to paste exactly as much as the first is, and a line to
  /// paste is a line with a copy button beside it.
  alternative?: Instruction;
};

/// One operating system's answers: what the tab is called, and an instruction
/// for each of the seven rows.
///
/// Where a session looks is not among them. That was a sentence per tab naming
/// the fixed list a session's `PATH` used to be; a session's `PATH` is now the
/// server's own, so the wizard draws the real list from the wire above the rows
/// — see `Dependencies.tsx`.
export type Guide = {
  /// What the tab is called.
  title: string;

  /// One instruction per row. Every row, on every OS: a tab with a gap in it is
  /// a row somebody is left staring at.
  rows: Record<Dependency, Instruction>;

  /// And what every command on this tab wants first, where they all want the
  /// same thing.
  ///
  /// One tab has one: the Apple-silicon Mac's, where every `brew install` line
  /// wants a Homebrew and a Mac without one has nothing to run them with. It is
  /// drawn above the rows rather than repeated under each of them, because it
  /// is one install for the whole tab — and it is here at all because a run
  /// that could not install Homebrew is exactly how somebody reaches this
  /// screen on a Mac.
  before?: Instruction;

  /// Or what this tab is, where what it is, is not what the machine beside it
  /// is.
  ///
  /// One tab has one of these too, and it is the other Mac's: two tabs saying
  /// *macOS* with different commands under them is a difference somebody has to
  /// be told about, and the difference is the whole of why the tab exists —
  /// Homebrew has dropped Intel. Drawn where the tab above draws Homebrew's own
  /// line, and a sentence rather than an instruction, because there is nothing
  /// here to run.
  about?: string;
};

/// The nine tabs, in the order they are drawn — the order
/// `verkstead_render::Distro` is written in, which is the two platforms, the
/// five distributions whose commands are written down, everything else, and the
/// second Mac at the end of it.
///
/// **A list a `Distro` can fall out of**, which nothing but a test can hold to
/// the type: a union of strings is gone by the time this runs, so a tab left
/// out here is a machine whose own commands are drawn nowhere and whose tab
/// nobody can press. What holds it is [`GUIDES`], which the type checker does
/// hold exhaustive — see `dependencies.test.tsx`, where the two are read
/// against each other.
export const DISTROS: readonly Distro[] = [
  "MacOs",
  "Windows",
  "NixOs",
  "Ubuntu",
  "Fedora",
  "Debian",
  "Arch",
  "OtherLinux",
  "MacOsIntel",
];

/// Claude Code the way Anthropic installs it, which is what every Linux tab
/// leads with.
///
/// **The packaged ones go stale, and this one does not.** A distribution's
/// `claude` can be too old to connect at all — Ubuntu's under WSL was — so the
/// install that stays current is the one to offer first, and it is the one most
/// people already have.
///
/// It lands in `~/.local/bin`, which a session reaches whenever the `PATH`
/// Verkstead was started with names it: the entry is kept and the directory
/// bound read-only, and the link into the versions directory is followed. See
/// `crates/server/src/sandbox.rs`'s `composed`. So what the note has to say is
/// the one thing nobody can read off the command — which shell's `PATH` has to
/// name it, and that Verkstead has to be started again after.
const CLAUDE_NATIVE: Instruction = {
  command: "curl -fsSL https://claude.ai/install.sh | bash",
  note:
    "Anthropic's own installer, and the one that stays current. It puts claude " +
    "in ~/.local/bin, so that directory has to be on the PATH of the shell " +
    "Verkstead is started from, with Verkstead started again once it is.",
};

/// And the same installer on a Mac, which is the same line with a note of its
/// own.
///
/// **A Mac says nothing about the shell's `PATH`, because it does not depend on
/// one.** Every other tab's note has to: what a session searches is the `PATH`
/// Verkstead was started with, so a directory that `PATH` never named is one no
/// session reaches. A Mac session's `PATH` is composed with the home's own
/// `.local/bin` at its head whichever way the app was started — see ADR-0016's
/// *Macs* — so there is nothing here for the human to put anywhere and nothing
/// to restart. It is the one row on any tab whose note is shorter than the
/// others rather than longer.
///
/// It is not an [`orElse`] either: Homebrew's `claude-code` cask was the Mac's
/// row for as long as `~/.local/bin` was somewhere the Dock's `PATH` could not
/// see, and with the floor carrying it the cask is a second install of the same
/// program that goes stale.
const CLAUDE_ON_A_MAC: Instruction = {
  command: "curl -fsSL https://claude.ai/install.sh | bash",
  note:
    "Anthropic's own installer, and the one that stays current. It puts " +
    "claude in ~/.local/bin, which is on every Mac session's PATH whichever " +
    "way Verkstead was started.",
};

/// The same installer on Windows, where it is the PowerShell one and the home
/// directory is spelled differently.
const CLAUDE_NATIVE_WINDOWS: Instruction = {
  command: "irm https://claude.ai/install.ps1 | iex",
  note:
    "Anthropic's own installer, and the one that stays current. It puts " +
    "claude.exe in %USERPROFILE%\\.local\\bin, so that directory has to be on " +
    "the PATH of the shell Verkstead is started from, with Verkstead started " +
    "again once it is.",
};

/// Grok Build, which is a link on every tab.
///
/// **The packages under that name are not xAI's.** The `grok-cli` in nixpkgs is
/// superagent-ai's agent and the `grok-cli` on npm is a proxy around claude-code;
/// either would install a different program under the name a session launches,
/// which is worse than having nothing to offer. So what is offered is the
/// vendor's own installer, and the caveat about where it puts the binary.
const GROK_UNIX: Instruction = {
  link: "https://x.ai/cli",
  note:
    "xAI's own installer puts grok in ~/.grok/bin and symlinks it into " +
    "/usr/local/bin where it can. Where it could not, ~/.grok/bin has to be on " +
    "the PATH of the shell Verkstead is started from. The grok-cli packaged by " +
    "nixpkgs and the one on npm are other people's projects rather than xAI's " +
    "grok.",
};

/// The same on Windows, where the installer is the PowerShell one.
const GROK_WINDOWS: Instruction = {
  link: "https://x.ai/cli",
  note:
    "irm https://x.ai/cli/install.ps1 | iex is xAI's own installer. The " +
    "grok-cli on npm is somebody else's project rather than xAI's grok.",
};

/// OpenCode the way OpenCode installs it, which is what the one tab with no
/// package manager on it has.
///
/// **It lands under the home and nowhere a Mac already looks.** Anthropic's
/// installer writes into `~/.local/bin`, which is on the Mac floor; this one
/// writes into `~/.opencode/bin`, which is on nobody's. So a tick is the way to
/// run it — Verkstead puts what it installed on every session's `PATH` — and a
/// run by hand is the sentence every other tab's note says: the directory has
/// to be on the `PATH` Verkstead is started from.
const OPENCODE_NATIVE: Instruction = {
  command: "curl -fsSL https://opencode.ai/install | bash",
  note:
    "OpenCode's own installer. It puts opencode in ~/.opencode/bin, which is " +
    "not a directory a Mac session looks in by itself: tick the row and " +
    "Verkstead puts it on every session's PATH, or run this by hand and put " +
    "that directory on the PATH of the shell Verkstead is started from, with " +
    "Verkstead started again once it is.",
};

/// A harness from npm, installed for the whole machine rather than for a user.
///
/// `-g` under the distribution's own node lands the binary in `/usr/local/bin`
/// or `/usr/bin`, both of them on the floor under every Linux session's `PATH`
/// — so this one needs nothing said about the `PATH` Verkstead was started
/// with, where an npm prefix pointed at somewhere under `$HOME` would.
function npm(pkg: string, node: string): Instruction {
  return {
    command: `sudo npm install -g ${pkg}`,
    note:
      "An -g install lands the binary in /usr/local/bin or /usr/bin, a system " +
      "directory a session reaches with nothing else to set. Where there is no " +
      `npm yet: ${node}.`,
  };
}

/// And the same on Windows, which has no `sudo` and takes its node from winget.
function windowsNpm(pkg: string): Instruction {
  return {
    command: `npm install -g ${pkg}`,
    note:
      "An -g install lands the binary in %APPDATA%\\npm, which Node's own " +
      "installer puts on your PATH. Where there is no npm yet: winget install " +
      "--id OpenJS.NodeJS.",
  };
}

/// The vendor's installer with this machine's own package kept under it.
///
/// Claude Code's row on the seven tabs that are not the Mac's: what leads is
/// the install that stays current, and what is under it is the one a machine
/// with a package manager may prefer.
function orElse(lead: Instruction, packaged: Instruction): Instruction {
  return { ...lead, alternative: packaged };
}

/// Everything a NixOS machine installs, which is a line in the system
/// configuration rather than a command.
function nixos(attribute: string, note?: string): Instruction {
  return {
    command: `environment.systemPackages = [ pkgs.${attribute} ];`,
    note:
      (note ? `${note} ` : "") +
      "In configuration.nix, then sudo nixos-rebuild switch. A nix profile " +
      "install goes to ~/.nix-profile/bin instead, which has to be on the PATH " +
      "of the shell Verkstead is started from.",
  };
}

/// The nine tabs' own answers.
///
/// Written out one tab at a time rather than composed out of a package manager
/// and a table of names: what the note under a row says is as much of the
/// instruction as the command is, and half of them are about the one machine
/// they are on.
export const GUIDES: Record<Distro, Guide> = {
  MacOs: {
    title: "macOS",

    // Homebrew's own line, as Homebrew publishes it. Verkstead installs it for
    // you where it can — the prefix made behind the password dialog and the
    // installer run as you — so this is what to paste on the Mac where that
    // could not be done: it asks for your password once, for the same prefix.
    //
    // What it is above is the rows that are a `brew install`, rather than all
    // of them: Claude Code is Anthropic's own installer here as everywhere
    // else, and Grok Build is xAI's.
    before: {
      command:
        '/bin/bash -c "$(curl -fsSL ' +
        'https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"',
      note:
        "The brew commands below are Homebrew's, and a Mac without Homebrew " +
        "has nothing to run them with. It installs into /opt/homebrew on " +
        "Apple silicon and /usr/local on Intel, and a session looks in both.",
    },

    rows: {
      Sandbox: {
        note:
          "Apple's own sandbox-exec is on every Mac, and it is what a session " +
          "runs inside here. There is nothing to install.",
      },
      // Apple's own dialog first, and Homebrew's git under it. Every Mac has a
      // /usr/bin/git and without the command line tools that file is a stub
      // that opens this same dialog instead of running — so this is the line
      // that makes the git the machine already has work, and it is the line
      // both Mac tabs lead with. Homebrew's is kept under it because it is the
      // git a developer's Mac usually runs, and a Mac reading this tab has a
      // brew to install it with.
      Git: {
        command: "xcode-select --install",
        note:
          "git comes with Apple's command line tools, and this opens Apple's " +
          "own dialog to install them. The git in /usr/bin without them is a " +
          "stub that opens the same dialog instead of running.",
        alternative: {
          command: "brew install git",
          note:
            "Homebrew's own, which is the git most Macs with brew run. It " +
            "wants the command line tools above installed first.",
        },
      },
      Claude: CLAUDE_ON_A_MAC,
      Codex: {
        command: "brew install --cask codex",
        note: "A cask rather than a formula.",
      },
      Grok: GROK_UNIX,
      OpenCode: { command: "brew install opencode" },
      Gh: { command: "brew install gh" },
    },
  },

  Windows: {
    title: "Windows",
    rows: {
      Sandbox: {
        command: "verkstead session-account create",
        note:
          "Sessions on this machine run as a local account of Verkstead's " +
          "own, which is the sandbox on Windows. Ticking this row makes it " +
          "for you: making a local account wants an administrator, and the " +
          "wizard asks Windows for one. This line is the same thing by hand, " +
          "from a terminal opened with Run as administrator, and it wants " +
          "running once. Until there is an account, a session on this " +
          "machine is refused rather than started without a boundary.",
      },
      Git: { command: "winget install --id Git.Git" },
      Claude: orElse(
        CLAUDE_NATIVE_WINDOWS,
        windowsNpm("@anthropic-ai/claude-code"),
      ),
      Codex: windowsNpm("@openai/codex"),
      Grok: GROK_WINDOWS,
      OpenCode: windowsNpm("opencode-ai"),
      Gh: { command: "winget install --id GitHub.cli" },
    },
  },

  NixOs: {
    title: "NixOS",
    rows: {
      Sandbox: nixos(
        "bubblewrap",
        "Verkstead's own NixOS module adds this line for you, so a module " +
          "install has nothing to do here. It is wanted in " +
          "environment.systemPackages even so, and not only on the service's " +
          "own PATH: what a session looks along is the machine's profile.",
      ),
      Git: nixos("git"),
      Claude: orElse(CLAUDE_NATIVE, nixos("claude-code")),
      Codex: nixos("codex"),
      Grok: GROK_UNIX,
      OpenCode: nixos("opencode"),
      Gh: nixos("gh"),
    },
  },

  Ubuntu: {
    title: "Ubuntu",
    rows: {
      Sandbox: { command: "sudo apt install bubblewrap" },
      Git: { command: "sudo apt install git" },
      Claude: orElse(
        CLAUDE_NATIVE,
        npm("@anthropic-ai/claude-code", "sudo apt install nodejs npm"),
      ),
      Codex: npm("@openai/codex", "sudo apt install nodejs npm"),
      Grok: GROK_UNIX,
      OpenCode: npm("opencode-ai", "sudo apt install nodejs npm"),
      Gh: { command: "sudo apt install gh" },
    },
  },

  Fedora: {
    title: "Fedora",
    rows: {
      Sandbox: { command: "sudo dnf install bubblewrap" },
      Git: { command: "sudo dnf install git" },
      Claude: orElse(
        CLAUDE_NATIVE,
        npm("@anthropic-ai/claude-code", "sudo dnf install nodejs npm"),
      ),
      Codex: npm("@openai/codex", "sudo dnf install nodejs npm"),
      Grok: GROK_UNIX,
      OpenCode: npm("opencode-ai", "sudo dnf install nodejs npm"),
      Gh: { command: "sudo dnf install gh" },
    },
  },

  Debian: {
    title: "Debian",
    rows: {
      Sandbox: { command: "sudo apt install bubblewrap" },
      Git: { command: "sudo apt install git" },
      Claude: orElse(
        CLAUDE_NATIVE,
        npm("@anthropic-ai/claude-code", "sudo apt install nodejs npm"),
      ),
      Codex: npm("@openai/codex", "sudo apt install nodejs npm"),
      Grok: GROK_UNIX,
      OpenCode: npm("opencode-ai", "sudo apt install nodejs npm"),
      Gh: { command: "sudo apt install gh" },
    },
  },

  Arch: {
    title: "Arch",
    rows: {
      Sandbox: { command: "sudo pacman -S bubblewrap" },
      Git: { command: "sudo pacman -S git" },
      Claude: orElse(
        CLAUDE_NATIVE,
        npm("@anthropic-ai/claude-code", "sudo pacman -S npm"),
      ),
      Codex: npm("@openai/codex", "sudo pacman -S npm"),
      Grok: GROK_UNIX,
      OpenCode: npm("opencode-ai", "sudo pacman -S npm"),
      Gh: { command: "sudo pacman -S github-cli" },
    },
  },

  /// A Linux naming none of the five: what is needed, in words, rather than a
  /// command that would be wrong on the machine it was pasted into.
  OtherLinux: {
    title: "Other Linux",
    rows: {
      Sandbox: {
        note:
          "Install your distribution's bubblewrap package — the program is " +
          "bwrap — and allow unprivileged user namespaces, which some kernels " +
          "ship switched off.",
      },
      Git: { note: "Install your distribution's git package." },
      Claude: orElse(
        CLAUDE_NATIVE,
        npm(
          "@anthropic-ai/claude-code",
          "install your distribution's nodejs and npm",
        ),
      ),
      Codex: npm("@openai/codex", "install your distribution's nodejs and npm"),
      Grok: GROK_UNIX,
      OpenCode: npm("opencode-ai", "install your distribution's nodejs and npm"),
      Gh: {
        note:
          "Install your distribution's GitHub CLI package, which is gh or " +
          "github-cli.",
      },
    },
  },

  /// And the Mac Homebrew has dropped, which is the tab drawn last.
  ///
  /// **Nothing here is a `brew install`**, so there is no Homebrew line above
  /// it: Homebrew's installer refuses an Intel Mac outright, and its formulae
  /// there get no bottles. What is left is what a Mac with no package manager
  /// can be told — Apple's own tools, the vendors' own installers, and a
  /// binary to unpack into a directory every Mac session looks in.
  MacOsIntel: {
    title: "macOS (Intel)",

    about:
      "Homebrew no longer supports Intel Macs — its installer refuses one " +
      "outright — so nothing on this tab is a brew install.",

    rows: {
      Sandbox: {
        note:
          "Apple's own sandbox-exec is on every Mac, and it is what a session " +
          "runs inside here. There is nothing to install.",
      },

      // Apple's own dialog rather than a package: every Mac has a
      // `/usr/bin/git`, and without the command line tools behind it that file
      // is a stub that opens this same dialog when a session runs it.
      Git: {
        command: "xcode-select --install",
        note:
          "git comes with Apple's command line tools, and this opens Apple's " +
          "own dialog to install them. The git in /usr/bin without them is a " +
          "stub that opens the same dialog instead of running.",
      },

      Claude: CLAUDE_ON_A_MAC,

      // Neither a script nor a package this machine can use: the npm the other
      // Unix tabs install it from wants a node this Mac has no package manager
      // to fetch, and the cask beside it is Homebrew's.
      Codex: {
        link: "https://github.com/openai/codex/releases",
        note:
          "Codex has no installer script, and the package the other tabs use " +
          "wants an npm this Mac has no package manager to install. Put the " +
          "binary in ~/.local/bin, which is on every Mac session's PATH " +
          "whichever way Verkstead was started.",
      },

      Grok: GROK_UNIX,
      OpenCode: OPENCODE_NATIVE,

      Gh: {
        link: "https://github.com/cli/cli/releases",
        note:
          "GitHub ships a zip for this Mac rather than a package it can " +
          "install. Unpack gh into ~/.local/bin, which is on every Mac " +
          "session's PATH whichever way Verkstead was started.",
      },
    },
  },
};

/// What the Windows sandbox row's line is, on the machine this reading is of.
///
/// **The one instruction that is about this server rather than about this
/// operating system.** Every other line on every tab is the same wherever it is
/// read; this one makes a local account named after a Data Directory, and the
/// verb resolves the platform's own default where nothing says otherwise — so a
/// line pasted without the directory makes an account of a different name,
/// leaves the row absent and says nothing about why. The row gates the step, so
/// that is the wizard stuck rather than advice that missed.
///
/// Which bites on exactly the machines this screen is for: a server that could
/// not raise a dialog is one started from a terminal or a unit file, and that is
/// the one most likely to have been pointed somewhere of its own.
///
/// `null` for a reading that names no directory, where the bare line is the best
/// there is to offer — see [`OnboardingView::data_directory`].
function theAccount(directory: string | null): Instruction {
  const bare = GUIDES.Windows.rows.Sandbox;

  if (directory === null) {
    return bare;
  }

  return { ...bare, command: `${bare.command} --data-dir "${directory}"` };
}

/// The instruction for one row on one tab, as this machine reads it.
///
/// The written-down answer for every row but one — see [`theAccount`], which is
/// the row this server has a word about.
export function instructionFor(
  distro: Distro,
  dependency: Dependency,
  directory: string | null,
): Instruction {
  return distro === "Windows" && dependency === "Sandbox"
    ? theAccount(directory)
    : GUIDES[distro].rows[dependency];
}
