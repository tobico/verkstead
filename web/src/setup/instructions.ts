//! What to run on this machine to get what a session needs — one set of
//! instructions per operating system, and one instruction per row.
//!
//! **The commands are exact, or there is no command.** A row on a machine whose
//! package manager carries the program says the line to paste; a row on one that
//! does not says where the vendor's own installer is and nothing else. What is
//! never written here is a command that would install a *different* program
//! under the right name — see [`GROK_UNIX`], which is the whole of why Grok
//! Build is a link on all eight tabs.
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
//! would rather have that. See [`Instruction`]'s `alternative`.
//!
//! **Eight tabs and not one**, because the detection is a guess. It comes off
//! `/etc/os-release`'s `ID` and then `ID_LIKE`, so a derivative names its parent
//! and something nobody has heard of names nothing: the detected tab opens and
//! the other seven stay a press away, for the machine the file was wrong about.
//!
//! Nothing here is on the wire. What the server says is what this machine *is*
//! and what it is missing; what to do about it is the same eight answers on
//! every Verkstead, so they are the viewer's own — see
//! `crates/render/src/onboarding.rs`, which carries the rows and the distro and
//! no prose at all.

import type { Dependency, Distro } from "../api/types";

/// One row's instruction, on one operating system.
///
/// A command, a link, or a note by itself — the sandbox row on macOS and on
/// Windows is nothing to install, and *other Linux* has the generic list of
/// what is needed where the five named distributions have a line to paste.
///
/// Nothing to install is not always nothing to do: the Windows sandbox row is
/// an account rather than a program, and its note carries the one elevated
/// command that makes one. A command field there would be a line to paste that
/// fails in the terminal most people have open, which is worse than a sentence
/// saying where to paste it.
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
};

/// The eight tabs, in the order they are drawn — the order
/// `verkstead_render::Distro` is written in, which is the two platforms, the
/// five distributions whose commands are written down, and everything else.
export const DISTROS: readonly Distro[] = [
  "MacOs",
  "Windows",
  "NixOs",
  "Ubuntu",
  "Fedora",
  "Debian",
  "Arch",
  "OtherLinux",
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

/// The eight tabs' own answers.
///
/// Written out one tab at a time rather than composed out of a package manager
/// and a table of names: what the note under a row says is as much of the
/// instruction as the command is, and half of them are about the one machine
/// they are on.
export const GUIDES: Record<Distro, Guide> = {
  MacOs: {
    title: "macOS",
    rows: {
      Sandbox: {
        note:
          "Apple's own sandbox-exec is on every Mac, and it is what a session " +
          "runs inside here. There is nothing to install.",
      },
      Git: {
        command: "brew install git",
        note:
          "Xcode's command line tools carry a git as well — xcode-select " +
          "--install — and either of the two is somewhere a session looks.",
      },
      Claude: {
        command: "brew install --cask claude-code",
        note:
          "A cask rather than a formula, and the install a Mac session finds " +
          "whichever way Verkstead was started. Anthropic's own installer — " +
          "curl -fsSL https://claude.ai/install.sh | bash — puts claude in " +
          "~/.local/bin instead, and an app started from the Dock has " +
          "launchd's PATH rather than a shell's, so it never names that " +
          "directory. Homebrew's prefix it always names.",
      },
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
        note:
          "Sessions on this machine run as a local account of Verkstead's " +
          "own, which is the sandbox on Windows: there is nothing to " +
          "install. Making that account is the one thing here that wants an " +
          "administrator, and it wants one once — verkstead session-account " +
          "create, from a terminal opened with Run as administrator. Until " +
          "it has been run, a session on this machine is refused rather " +
          "than started without a boundary.",
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
};
