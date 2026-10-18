//! The mark an **Annex** wears beside its name: *annex of* its Keeper.
//!
//! Here rather than in either page because two draw it — the Devices list,
//! where it is how an Annex's row says whose it is, and the compose page's
//! device select, where an Annex carries its Keeper's hostname and would read
//! exactly as its Keeper's row does without it. One mark drawn two ways would
//! be two things to learn.

import type { JSX } from "solid-js";

import type { DeviceIdentity, DevicesView } from "./api/types";
import styles from "./AnnexMark.module.css";

/// What an **Annex**'s mark says whose it is: its Keeper's name as the cluster
/// holds it, or *this device* on the Keeper itself — `null` for a device that
/// is not an Annex.
///
/// A Keeper the reading holds no row for — a cluster still settling — is the
/// empty string, which the mark draws as *annex* alone rather than naming a
/// device by its id.
export function keeperOf(list: DevicesView, of: DeviceIdentity): string | null {
  const keeper = of.kept_by;

  if (keeper === null) return null;
  if (keeper === list.this.device) return "this device";

  return (
    list.members.find((member) => member.identity.device === keeper)?.identity
      .name ?? ""
  );
}

/// The mark itself, off what [`keeperOf`] answered — and the caller's own
/// class, for where it sits beside the name.
export function AnnexMark(props: { keeper: string; class?: string }): JSX.Element {
  return (
    <span class={[styles.annex, props.class].filter(Boolean).join(" ")}>
      {props.keeper ? `annex of ${props.keeper}` : "annex"}
    </span>
  );
}
