//! Why a load did not happen, in words the log file can hold.
//!
//! **Because the window is loaded on the login link, and Electron puts the URL
//! it was given into the failure.** A rejected `loadURL` carries a message of
//! the form `ERR_CONNECTION_REFUSED (-102) loading '<the url>'`, so a line
//! built by stringifying that error is the **Workbench Key** written into
//! `verkstead.log` — the file **View Logs** opens on somebody's desk and the
//! file somebody reporting a problem is asked to attach (ADR-0015). One
//! `String(trouble)` is all it takes, which is why the wording is a function
//! here rather than an interpolation at the call site: this is the piece that
//! must not carry a secret, so it is the piece with a test over it.
//!
//! **The fields rather than the message.** Electron hangs `code` and `errno` on
//! that error beside the message it composed — the description and the number,
//! the two halves worth reporting — and neither of them is ever a URL. So the
//! line is built from those, and the address the caller already knows is said
//! by the caller. A failure that carries neither is something other than a
//! refused navigation, and it is named as such rather than printed.

/// What a rejected navigation says about itself, which is as much of one as
/// this has any business reading.
///
/// A value rather than an `Error`: both fields are Electron's own additions
/// rather than anything the language puts there, so a caller handing over
/// something else is the ordinary case and not a misuse.
interface Refused {
  /// The description — `ERR_CONNECTION_REFUSED` and its like.
  code?: unknown;
  /// And the number behind it, which is what an Electron issue is searched by.
  errno?: unknown;
}

/// Why `trouble` stopped a load, said without whatever URL it was carrying.
///
/// The description and the number where the failure has them, the description
/// alone where it has only that, and a flat refusal to print anything where it
/// has neither — because what is left in that case is an error whose whole
/// account of itself is a message this must not repeat.
export function why(trouble: unknown): string {
  if (typeof trouble !== "object" || trouble === null) {
    return "the reason was nothing this can report without repeating it";
  }

  const { code, errno } = trouble as Refused;

  if (typeof code !== "string" || code === "") {
    return "the reason was nothing this can report without repeating it";
  }

  return typeof errno === "number" ? `${code} (${errno})` : code;
}
