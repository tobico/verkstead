//! The mark an **Annex** wears beside its name: *annex of* its Keeper.
//!
//! The Devices list draws it and nothing else does. Whose a device is belongs
//! on the page about the devices, which is where somebody looking for it goes;
//! the device select, which is a page asking *which machine*, says a machine's
//! name and the mark for its operating system and leaves it there (see
//! `annexed` in [`./workbench/Setup.tsx`](./workbench/Setup.tsx)).
//!
//! Here rather than inside that page all the same, because [`keeperOf`] is the
//! reading rather than the drawing: what an Annex's row says whose it is comes
//! off the cluster, and the select reads it too to know an Annex when it sees
//! one.

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
