//! Which Verkstead a page is reaching: this device, or a member of its cluster.
//!
//! A Conversation lives on the device that is doing its work, and every one of
//! them is reachable through any other: the browser asks the device it opened,
//! that device puts the call to the member over the Peer Listener, and the
//! answer comes back untouched (ADR-0020, *The opened device relays*). So the
//! client stays same-origin and holds one cookie, and the only thing that is
//! ever different about a remote Conversation is **which device it is about**.
//!
//! That is what this module is: the one value the whole viewer's device
//! dimension is made of, where a page reads it, and the two things it changes.
//!
//! - **A path.** `/api/ui/…` becomes `/api/ui/members/{device}/…`, which is done
//!   in one place — see `api/client.ts`. Nothing else composes a path.
//! - **A query key.** See [`keyOf`]: ids are each device's own and collide by
//!   construction, so a key holding a bare Conversation id would reach the wrong
//!   Conversation the moment two devices are in play.
//! - **And what a row *is*.** See [`rowKey`]: the sidebar is one list merged
//!   from every device in the cluster, so a row is a Conversation and a device
//!   rather than a number — which row is selected, which one a press was on,
//!   what a drag is holding, and which entry an overlay is about are all keyed
//!   that way.
//!
//! **Read off the URL and handed down**, rather than passed from component to
//! component: a pane that talks to the server needs it, and nearly every pane
//! between the page and that one does not. `null` is the whole of what a page
//! outside any provider gets, which is what makes every local page — the compose
//! page, the settings — the page it has always been without a line changed in
//! it.
//!
//! **The sidebar is the one thing that is about no single device**, and so the
//! one that is handed the value rather than reading it: its list is merged from
//! the whole cluster, so each of its rows carries its own device and every press
//! on one is addressed by that rather than by the page around it.

import { createContext, useContext } from "solid-js";
import type { QueryKey } from "@tanstack/solid-query";

import type { RowDevice } from "./api/types";

/// Which device something is about: a member of this device's cluster, by the
/// Device Id the URL carries, or `null` for this device itself.
///
/// `null` rather than an absent field, so that *this device* is a value like any
/// other and every seam below has one shape rather than two.
export type Device = string | null;

/// Which device the page a component is drawn on is about.
///
/// An accessor rather than a value, because it changes under a subtree that
/// stays up: walking from a local Conversation to a remote one is a navigation
/// and nothing more.
export const Reaching = createContext<() => Device>(() => null);

/// Which device the page around this component is about — `null` for this one,
/// which is what a page that never said is.
export function useDevice(): () => Device {
  return useContext(Reaching);
}

/// A query key for what one device holds.
///
/// The device leads the key where there is one, so that everything of a
/// member's is one prefix — what a re-announced member nudge invalidates whole —
/// and every key of this device's own is the key it has always been.
///
/// Which is the point of keying at all: **ids collide by construction.** Every
/// Verkstead issues a Conversation 1, so B's Conversation 4 and this device's
/// Conversation 4 are one entry in the cache unless something says they are two,
/// and one entry drawn twice is the wrong Timeline on the screen.
export function keyOf(device: Device, ...key: QueryKey): QueryKey {
  return device === null ? key : [device, ...key];
}

/// Which Conversation something is about, as one string: the device and the id
/// together.
///
/// **For the same reason [`keyOf`] exists, one list along.** The sidebar is
/// merged from every device in the cluster (ADR-0020, *The opened device
/// relays*), so a bare id names a row on no particular machine: B's
/// Conversation 4 and this device's Conversation 4 are two rows of one list, and
/// anything that told them apart by the number alone would act on whichever it
/// found first. So this is what a row *is* wherever the sidebar holds one — the
/// list's reconcile key aside, which is the row's Rank — and it is what a press
/// on one is looked up under.
///
/// This device's own read the way they always have, the empty device leading:
/// the point is that two devices never write the same string, and nothing about
/// it is ever shown to anybody.
export function rowKey(device: Device, conversation: number): string {
  return `${device ?? ""}/${conversation}`;
}

/// Which device a sidebar row belongs to.
///
/// The block on the row is the server's: the name and the OS word for drawing
/// it, and the Device Id for reaching it. `null` is this device — both where the
/// block says so and where there is no block at all, which is a device with no
/// cluster and so nothing to say about whose work it is drawing.
export function whose(row: { device: RowDevice | null }): Device {
  return row.device?.id ?? null;
}
