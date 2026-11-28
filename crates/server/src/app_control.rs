//! Whether Smart App Control is on, on the Windows machine this server is
//! running on — the one read, and the reason it is worth reading.
//!
//! **It is what stopped a session running what it built.** A Windows session
//! compiles in its own Worktree and then cannot start the executable it just
//! produced: Windows refuses it with *An Application Control policy has blocked
//! this file*. What refuses it is this setting, which enforces that a program be
//! signed by somebody Microsoft's own reputation service vouches for — and a
//! binary a minute old is nobody's reputation. So an agent changing anything
//! visual on that machine could not see its own result, and every check by eye
//! went round the human.
//!
//! **And nothing Verkstead can do reaches it.** The policy takes no allow rule
//! scoped to a directory, and it trusts no certificate added on the machine, so
//! neither an exception for the worktrees nor signing a session's build output
//! is a way through. The one way through is the human turning it off in Windows
//! Security, which they cannot undo without reinstalling Windows. That makes
//! this a thing to say rather than a thing to fix: Verkstead reads the state,
//! says what it means for sessions in the three places below, and never writes
//! it.
//!
//! **Read off the machine's own code-integrity policy, unprivileged.**
//! `HKLM\SYSTEM\CurrentControlSet\Control\CI\Policy` holds
//! `VerifiedAndReputablePolicyState`, which is `1` where the setting is
//! enforcing, `2` where it is evaluating and `0` or absent where it is off. A
//! read and nothing else, which is the stance everything the server does to
//! this machine stands under — see [`crate::sandbox::account`], whose own
//! registry read is the precedent for this one.
//!
//! **One read, three places that draw it.** The wizard's machine step says
//! what the state is while somebody is standing at the box; the settings page's
//! Sandbox section says it long after, which is where a machine that has
//! hardened since onboarding is read; and a Windows session is told, where the
//! setting is enforcing, that the binaries it builds will not launch here. The
//! session is told in that one state alone: evaluation blocks nothing today,
//! and a session warned about a block it will not hit is a session spending its
//! context on a condition that is not there.

use verkstead_render::AppControl;

/// Whether Smart App Control is on, as this machine answers now.
///
/// `None` on every machine it is off on, and on every machine that has no such
/// setting — which is both Unixes, where this is one branch and no call at all.
///
/// **Asked by each of the three that ask, at the moment it asks.** A session's
/// priming asks here at every launch; the wizard's machine step and the
/// settings page ask at every reading they are drawn from — see
/// [`crate::onboarding::Machine::app_control`], which is the shape that
/// machine's `PATH` has for the same reason.
///
/// Nothing holds an answer, because this is the one state Verkstead tells the
/// human to go and change: a reading kept from startup would be the settings
/// page going on saying a session cannot run what it builds after they had
/// turned the setting off on the strength of its saying so. One registry read,
/// which is what makes asking every time affordable.
pub fn of_this_machine() -> Option<AppControl> {
    #[cfg(windows)]
    return read_the_policy().and_then(stated);

    #[cfg(not(windows))]
    None
}

/// What the policy's own number means, which is the whole of the mapping.
///
/// Its own function so the three states can be asserted on a machine that has
/// no such policy to read: what is being decided is which number is which
/// state, and that is not a fact about the machine running the test.
#[cfg_attr(not(windows), allow(dead_code))]
fn stated(said: u32) -> Option<AppControl> {
    match said {
        1 => Some(AppControl::Enforcing),
        2 => Some(AppControl::Evaluation),
        // Nought is off, and so is anything this release has not heard of: a
        // number nobody has documented is not a reason to tell the human their
        // sessions cannot run what they build.
        _ => None,
    }
}

/// `VerifiedAndReputablePolicyState`, or nothing at all.
///
/// Nothing is a machine with no such value — a Windows old enough never to have
/// had the setting, or a machine where it has never been turned on — and a read
/// that failed for any other reason, which is the same answer: this is a
/// warning, and a warning that could not be read is one not drawn.
#[cfg(windows)]
fn read_the_policy() -> Option<u32> {
    use std::ffi::OsStr;

    use windows_sys::Win32::System::Registry::{
        HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RegGetValueW,
    };

    use crate::sandbox::starting::wide;

    /// Where the machine keeps the state of its code-integrity policies.
    const POLICY: &str = r"SYSTEM\CurrentControlSet\Control\CI\Policy";

    /// And the one value of it this reads: whether what may run here is held to
    /// being verified and reputable.
    const VERIFIED_AND_REPUTABLE: &str = "VerifiedAndReputablePolicyState";

    let mut said = 0u32;
    let mut room = size_of::<u32>() as u32;

    // Asked for as a `REG_DWORD` and nothing else, so a value of some other
    // type is read as no answer rather than as four bytes of something else.
    let read = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            wide(OsStr::new(POLICY)).as_ptr(),
            wide(OsStr::new(VERIFIED_AND_REPUTABLE)).as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            std::ptr::from_mut(&mut said).cast(),
            &mut room,
        )
    };

    (read == 0).then_some(said)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Which number is which state, asserted on any machine: the mapping is a
    /// decision rather than a reading, and it is the one part of this that
    /// could be wrong in a way nothing else notices.
    ///
    /// Verified against the real value on a Windows machine before it was
    /// relied on: dragon, whose setting the human had turned off, reads `0`
    /// with `SAC_PreviousState` at `1` beside it, which is the enforcing state
    /// it was turned off from.
    #[test]
    fn the_policy_says_enforcing_evaluation_or_nothing_to_warn_about() {
        assert_eq!(stated(1), Some(AppControl::Enforcing));
        assert_eq!(stated(2), Some(AppControl::Evaluation));
        assert_eq!(stated(0), None, "nought is the setting turned off");

        for unheard_of in [3, 4, 255] {
            assert_eq!(
                stated(unheard_of),
                None,
                "a state this release has not heard of is nothing to warn about \
                 rather than a guess at which of the two it is"
            );
        }
    }

    /// And nothing is read at all off a machine that has no such setting.
    #[cfg(not(windows))]
    #[test]
    fn a_machine_that_is_not_windows_has_no_app_control_to_read() {
        assert_eq!(of_this_machine(), None);
    }
}
