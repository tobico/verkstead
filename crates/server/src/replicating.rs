//! The **Replicated records** on the server's side: the kinds this server
//! declares on the store's facility, and what each one's writes hand on to
//! whatever reads them (ADR-0028).
//!
//! The record — a uuid, an `updated_at`, a deletion stamp and the device that
//! wrote it — and the two rules a record is merged and stamped by are the
//! store's, said once for every kind — see [`store::replicated`]. What lives
//! here is each kind's own: what its fields are, where it was kept before it
//! was a record, and who holds it in memory. Authors are the first.
//!
//! Named for what it does rather than `records`, which is OpenCode's store
//! reader's.

pub mod authors;
