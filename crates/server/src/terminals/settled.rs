//! Whether a Conversation's terminals have something running in them, as the
//! sidebar's card draws it (ADR-0025): the [`super::busy`] reading taken on a
//! turn, and held until it has said the same thing twice.
//!
//! **Held on the way on and not on the way off.** A terminal that has just
//! opened reads busy for its first half-second, the sandbox wrapper being in
//! front of the shell until the shell has come up and taken the terminal for
//! itself, and a card that lit for that would be an icon flashing on every
//! **New terminal**. So the reading goes on only once two turns running have
//! read it busy, which a shell coming up never does, and off on the first turn
//! that reads it quiet: a build that has finished is finished, and an icon
//! still saying it runs is the one way round this could be wrong that the
//! human would act on.

/// One Conversation's reading, turn by turn.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Settled {
    /// Whether the last turn read anything running, which is half of what the
    /// next turn needs to light it.
    once: bool,

    /// And what the card draws: on once two turns running read busy, off on
    /// the first that does not.
    busy: bool,
}

impl Settled {
    /// What the card draws.
    pub(crate) fn busy(self) -> bool {
        self.busy
    }

    /// One turn's reading, and whether what the card draws moved with it.
    pub(crate) fn read(&mut self, busy: bool) -> bool {
        let was = self.busy;

        self.busy = busy && (self.once || self.busy);
        self.once = busy;

        self.busy != was
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One turn reading busy is a shell still coming up, and lights nothing.
    #[test]
    fn one_busy_turn_lights_nothing() {
        let mut settled = Settled::default();

        assert!(!settled.read(true));
        assert!(!settled.busy());
    }

    /// Two running is something running, and the second is the flip.
    #[test]
    fn two_busy_turns_running_light_it() {
        let mut settled = Settled::default();

        settled.read(true);

        assert!(settled.read(true));
        assert!(settled.busy());

        // And it stays lit without moving again.
        assert!(!settled.read(true));
        assert!(settled.busy());
    }

    /// A quiet turn between two busy ones starts the count again.
    #[test]
    fn a_quiet_turn_between_starts_again() {
        let mut settled = Settled::default();

        settled.read(true);
        settled.read(false);

        assert!(!settled.read(true));
        assert!(!settled.busy());
    }

    /// And the first quiet turn puts it out.
    #[test]
    fn the_first_quiet_turn_puts_it_out() {
        let mut settled = Settled::default();

        settled.read(true);
        settled.read(true);

        assert!(settled.read(false));
        assert!(!settled.busy());

        // And one busy turn after that is the count starting again.
        assert!(!settled.read(true));
        assert!(!settled.busy());
    }
}
