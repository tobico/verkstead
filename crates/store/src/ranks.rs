//! The keys the sidebar is ordered by: a **Rank** per Conversation (ADR-0020,
//! *Ranks*).
//!
//! A rank is a **fractional-indexing key** with the device that issued it
//! suffixed after it, and the whole of what this module does is mint one: *a
//! rank strictly between these two, either of which may be absent*, where
//! nothing above means above everything and nothing below means below
//! everything. Ordering by rank is what the sidebar's order comes to: a row is
//! moved by writing one string on one row rather than by renumbering the whole
//! list, which is what will let a drag on a merged list be written to the device
//! that owns the row and nothing else.
//!
//! **The order-by is still the places**, this being the stage that mints the
//! keys rather than the one that reads them — see [`super::placements`]. What is
//! here is a rank on every row there is, and what reads them comes next.
//!
//! **Fractional indexing rather than a dense integer**, for the reason the ADR
//! gives: a key between any two always exists, so there is no arrangement a drag
//! cannot express, and the keys grow only where somebody keeps inserting in one
//! spot — no bucket to size and no rebalance pass to run.
//!
//! **The published scheme rather than a variant of it.** Base62 over
//! `0-9A-Za-z`, with the leading character encoding the length of the integer
//! part: `a` is two characters, `b` three, and `Z` is two counting downwards,
//! `Y` three. That head is what makes ranking *above the current top* unbounded
//! and compact — `a0`, then `Zz`, `Zy`, … `Z0`, then `Yzz` — and ranking above
//! everything is this feature's hot path, since it happens at every start. The
//! arithmetic is written here rather than taken as a dependency because both
//! ends of the wire need it and an alphabet the two disagreed about would be two
//! orders; what makes writing it safe is the property test at the foot of this
//! module, which asserts a few thousand random inserts each strictly between its
//! neighbours with the whole list still in order afterwards.
//!
//! **A rank is the key, a separator, and this device's id.** The separator has
//! to sort *below* every character of the alphabet, and [`SEPARATOR`] does — `0`
//! being the lowest character the alphabet has — which buys two things at once.
//! Ordering the suffixed strings is ordering by key and then by device, so two
//! devices that minted the same key are still a stated order rather than a tie;
//! and a key minted between two neighbours still sorts between them once every
//! suffix is on, because wherever one key is a prefix of another the separator
//! is what the shorter one continues with.
//!
//! **The arithmetic never sees a suffix.** It comes off before the key is
//! computed and the new one goes back on after — a suffix is not a character of
//! the alphabet, and an integer part read through one would be a different
//! length.

use anyhow::{Context, Result, bail, ensure};

/// The alphabet a key is written in: base62, in ascending order, which is also
/// ASCII order — that is what makes comparing two keys a string comparison.
const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// What stands between a key and the device that issued it.
///
/// `-` because it sorts below every character of [`DIGITS`] — see the module
/// docs, where both things that buys are set out. Nothing else about it is load
/// bearing: it is not in the alphabet, so it is also what tells the key from the
/// suffix when a rank is read back.
const SEPARATOR: char = '-';

/// The lowest key the scheme has: the shortest negative head with nothing but
/// zeroes under it.
///
/// Nothing can be ranked above it, so it is never minted — [`key_between`]
/// refuses to hand one out and works inside it instead, exactly as the published
/// scheme does. Reaching it would take twenty-six carries of the integer part,
/// which is more insertions at the top of one sidebar than a human has presses
/// in them.
const SMALLEST: &str = "A00000000000000000000000000";

/// A rank strictly between `above` and `below`, suffixed with `device`.
///
/// `above` is the rank of the row above in the sidebar and `below` the rank of
/// the row below, so `None` above means *above everything* and `None` below
/// means *below everything*; both absent is the only row there is. One call for
/// all three uses: a Conversation is ranked above the top at the moment it is
/// started, the rewrite ranks a database's rows one below the last, and a drag
/// will rank the moved row between the two it landed between.
///
/// The suffix is taken off each neighbour before the key is computed and
/// `device`'s is put on the one that comes back — see the module docs. So the
/// rank that comes out is this device's whoever the neighbours belong to, which
/// is what makes a drag on a merged list a write to the row's owner alone.
///
/// An error is a rank that will not parse, a pair the wrong way round, or a
/// neighbour pair with nothing between them at all — which is two rows at the
/// same key, the one thing the suffix is there to stop happening.
pub fn between(above: Option<&str>, below: Option<&str>, device: &str) -> Result<String> {
    let key = key_between(above.map(key_of), below.map(key_of)).with_context(|| {
        match (above, below) {
            (Some(above), Some(below)) => format!("ranking between {above} and {below}"),
            (Some(above), None) => format!("ranking below {above}"),
            (None, Some(below)) => format!("ranking above {below}"),
            (None, None) => "ranking the first Conversation".to_owned(),
        }
    })?;

    Ok(format!("{key}{SEPARATOR}{device}"))
}

/// The key half of a rank: everything before the separator, or the whole of it
/// where there is none.
///
/// A rank with no suffix is not something this writes, and it is read rather
/// than refused all the same: the arithmetic validates what it is given, so a
/// string that is not a key fails where it is used rather than here.
fn key_of(rank: &str) -> &str {
    rank.split_once(SEPARATOR).map_or(rank, |(key, _)| key)
}

/// The published scheme's own function: a key strictly between two keys, either
/// of which may be absent.
///
/// This is the whole of the arithmetic, and it is the part with no notion of a
/// device: what it sees are keys over [`DIGITS`] and what it hands back is
/// another one.
fn key_between(above: Option<&str>, below: Option<&str>) -> Result<String> {
    if let Some(above) = above {
        validate(above)?;
    }

    if let Some(below) = below {
        validate(below)?;
    }

    match (above, below) {
        (Some(above), Some(below)) if above == below => bail!(
            "{above} is both of them: nothing sorts strictly between two rows ranked at the same \
             key, which is what the device suffix is there to keep from happening"
        ),
        (Some(above), Some(below)) => {
            ensure!(above < below, "{above} does not sort above {below}");
        }
        _ => {}
    }

    match (above, below) {
        // The only row there is, which is the shortest key the scheme has a
        // positive head for.
        (None, None) => Ok(format!("a{}", DIGITS[0] as char)),

        // Above everything: the integer part on its own where there is a
        // fractional part to drop, and the integer part one lower where there
        // is not.
        (None, Some(below)) => {
            let integer = integer_part(below)?;
            let fraction = &below[integer.len()..];

            if integer == SMALLEST {
                return Ok(format!("{integer}{}", midpoint("", Some(fraction))?));
            }

            if integer < below {
                return Ok(integer.to_owned());
            }

            decrement(integer)?.context("there is no key left above this one")
        }

        // And below everything: the integer part one higher, or a fraction
        // under it where the integer part has run out of room.
        (Some(above), None) => {
            let integer = integer_part(above)?;
            let fraction = &above[integer.len()..];

            match increment(integer)? {
                Some(next) => Ok(next),
                None => Ok(format!("{integer}{}", midpoint(fraction, None)?)),
            }
        }

        // Between two: the midpoint of their fractions where they share an
        // integer part, and otherwise the next integer part where that still
        // falls short of the row below.
        (Some(above), Some(below)) => {
            let over = integer_part(above)?;
            let over_fraction = &above[over.len()..];
            let under = integer_part(below)?;
            let under_fraction = &below[under.len()..];

            if over == under {
                return Ok(format!(
                    "{over}{}",
                    midpoint(over_fraction, Some(under_fraction))?
                ));
            }

            let next = increment(over)?.context("there is no key left below this one")?;

            if next.as_str() < below {
                Ok(next)
            } else {
                Ok(format!("{over}{}", midpoint(over_fraction, None)?))
            }
        }
    }
}

/// A fraction strictly between two fractions, `None` standing for the end of the
/// range rather than for a fraction of nothing.
///
/// A fraction is the part of a key after its integer part: base62 digits with no
/// trailing zero, read as digits after a point. Halving the interval is what
/// makes a key always available between two others, and taking the longest
/// common prefix off first is what keeps the result as short as the interval
/// allows.
fn midpoint(above: &str, below: Option<&str>) -> Result<String> {
    if let Some(below) = below {
        ensure!(above < below, "{above} does not sort above {below}");
    }

    // A trailing zero is the one thing a fraction may not have: `1` and `10`
    // would be the same number and two different strings, so every key would
    // have as many spellings as it has zeroes to spare.
    ensure!(
        !above.ends_with('0'),
        "{above} is a fraction with a trailing zero"
    );
    ensure!(
        !below.is_some_and(|below| below.ends_with('0')),
        "a fraction with a trailing zero"
    );

    if let Some(below) = below {
        // The longest prefix they share, a digit the shorter one has run out of
        // counting as a zero — which is what it is, a fraction being read as
        // digits after a point.
        let over = above.as_bytes();
        let under = below.as_bytes();
        let mut shared = 0;

        while shared < under.len() && *over.get(shared).unwrap_or(&b'0') == under[shared] {
            shared += 1;
        }

        if shared > 0 {
            let rest = above.get(shared..).unwrap_or("");

            return Ok(format!(
                "{}{}",
                &below[..shared],
                midpoint(rest, Some(&below[shared..]))?
            ));
        }
    }

    let over = match above.as_bytes().first() {
        Some(&digit) => value(digit)?,
        None => 0,
    };
    let under = match below {
        Some(below) => value(below.as_bytes()[0])?,
        None => DIGITS.len(),
    };

    // Room between the two leading digits: halfway between them is a fraction of
    // one digit, which is as short as anything in this interval can be.
    if under - over > 1 {
        let halfway = (0.5 * (over + under) as f64).round() as usize;

        return Ok((DIGITS[halfway] as char).to_string());
    }

    // And where they are consecutive digits there is nothing between the two at
    // this position, so the answer is a digit longer: the row below's own
    // leading digit where it has more under it, and otherwise the row above's,
    // with the interval below *that* halved in turn.
    match below {
        Some(below) if below.len() > 1 => Ok(below[..1].to_owned()),
        _ => Ok(format!(
            "{}{}",
            DIGITS[over] as char,
            midpoint(above.get(1..).unwrap_or(""), None)?
        )),
    }
}

/// How long the integer part of a key whose leading character is `head` is,
/// counting the head itself.
///
/// The head is the length rather than the length being counted or delimited,
/// which is what lets two keys be compared as strings: `a` is two characters and
/// `z` twenty-seven, counting up, and `Z` is two and `A` twenty-seven, counting
/// down. So a longer positive key sorts below a shorter one and a longer
/// negative key above it, which is exactly what the arithmetic needs of them.
fn integer_length(head: u8) -> Result<usize> {
    if head.is_ascii_lowercase() {
        Ok(usize::from(head - b'a') + 2)
    } else if head.is_ascii_uppercase() {
        Ok(usize::from(b'Z' - head) + 2)
    } else {
        bail!("{} is not the head of a key", head as char)
    }
}

/// The integer part of a key: its head, and the digits its head says follow.
fn integer_part(key: &str) -> Result<&str> {
    let head = *key
        .as_bytes()
        .first()
        .context("an empty string is not a key")?;
    let length = integer_length(head)?;

    ensure!(
        key.len() >= length,
        "{key} is shorter than the {length} characters its head says it has"
    );

    Ok(&key[..length])
}

/// Whether `key` is one this arithmetic can work from.
///
/// Three things are asked of it, and each of them is something a key this module
/// minted cannot fail: its head says a length it is long enough for, every
/// character after the head is in the alphabet, and its fraction has no trailing
/// zero. What a failure means is a rank from somewhere else — a hand-edited row,
/// or a device that agreed about the alphabet and nothing else.
fn validate(key: &str) -> Result<()> {
    ensure!(
        key != SMALLEST,
        "{key} is the lowest key there is, so nothing can be ranked above it"
    );

    let integer = integer_part(key)?;

    for &digit in &key.as_bytes()[1..] {
        value(digit)?;
    }

    ensure!(
        !key[integer.len()..].ends_with('0'),
        "{key} has a trailing zero, which is a second spelling of a shorter key"
    );

    Ok(())
}

/// Where `digit` sits in the alphabet.
fn value(digit: u8) -> Result<usize> {
    DIGITS
        .iter()
        .position(|&candidate| candidate == digit)
        .with_context(|| format!("{} is not a digit of a key", digit as char))
}

/// The integer part one higher than `integer`, carrying into the head where the
/// digits run out.
///
/// `None` is the end of the scheme: twenty-seven `z` digits, past which there is
/// no longer positive head to carry into. Nothing here reaches it — a key is
/// only incremented to rank below the bottom row, and the bottom row would have
/// to have been ranked there first.
fn increment(integer: &str) -> Result<Option<String>> {
    let head = integer.as_bytes()[0];
    let mut digits = integer.as_bytes()[1..].to_vec();
    let mut carry = true;

    for digit in digits.iter_mut().rev() {
        if !carry {
            break;
        }

        let next = value(*digit)? + 1;

        if next == DIGITS.len() {
            *digit = DIGITS[0];
        } else {
            *digit = DIGITS[next];
            carry = false;
        }
    }

    if !carry {
        return Ok(Some(headed(head, &digits)));
    }

    // The digits all rolled over, so the head moves. Crossing from the negative
    // heads to the positive ones is the one step that is not a head away: `Z`
    // is the last of the negatives and `a` the first of the positives, and what
    // follows `Zz` counting upwards is `a0`.
    if head == b'Z' {
        return Ok(Some(format!("a{}", DIGITS[0] as char)));
    }

    if head == b'z' {
        return Ok(None);
    }

    let head = head + 1;

    // A positive head one higher is one digit longer, and a negative head one
    // higher is one digit shorter — see [`integer_length`].
    if head > b'a' {
        digits.push(DIGITS[0]);
    } else {
        digits.pop();
    }

    Ok(Some(headed(head, &digits)))
}

/// The integer part one lower than `integer`, borrowing from the head where the
/// digits run out.
///
/// `None` is the other end of the scheme: [`SMALLEST`], past which there is no
/// longer negative head to borrow from. What reaches it is ranking above
/// everything twenty-six carries in a row, and [`key_between`] is what says so.
fn decrement(integer: &str) -> Result<Option<String>> {
    let head = integer.as_bytes()[0];
    let mut digits = integer.as_bytes()[1..].to_vec();
    let mut borrow = true;

    for digit in digits.iter_mut().rev() {
        if !borrow {
            break;
        }

        match value(*digit)?.checked_sub(1) {
            Some(next) => {
                *digit = DIGITS[next];
                borrow = false;
            }
            None => *digit = DIGITS[DIGITS.len() - 1],
        }
    }

    if !borrow {
        return Ok(Some(headed(head, &digits)));
    }

    // And the same crossing the other way: what follows `a0` counting downwards
    // is `Zz`.
    if head == b'a' {
        return Ok(Some(format!("Z{}", DIGITS[DIGITS.len() - 1] as char)));
    }

    if head == b'A' {
        return Ok(None);
    }

    let head = head - 1;

    if head < b'Z' {
        digits.push(DIGITS[DIGITS.len() - 1]);
    } else {
        digits.pop();
    }

    Ok(Some(headed(head, &digits)))
}

/// A head and its digits, back as the string they are.
fn headed(head: u8, digits: &[u8]) -> String {
    let mut key = String::with_capacity(digits.len() + 1);

    key.push(head as char);
    key.extend(digits.iter().map(|&digit| digit as char));

    key
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The device every rank here is suffixed with, by the id a cluster names
    /// one by.
    const THIS_DEVICE: &str = "0011223344556677889900aabbccddee";

    /// And a second one, for the rows a merged list would hold from somewhere
    /// else.
    const ANOTHER_DEVICE: &str = "ffeeddccbbaa00998877665544332211";

    /// The shape the scheme is chosen for: ranking above everything is one
    /// character of growth every sixty-two presses rather than a rewrite of the
    /// list, and the keys it walks through are the published ones.
    #[test]
    fn ranking_above_everything_walks_down_the_heads() {
        let mut minted = Vec::new();
        let mut top = None;

        for _ in 0..3 {
            let rank = between(None, top.as_deref(), THIS_DEVICE).unwrap();

            minted.push(key_of(&rank).to_owned());
            top = Some(rank);
        }

        assert_eq!(
            minted,
            vec!["a0", "Zz", "Zy"],
            "the first key, then the heads counting downwards",
        );
    }

    /// And the sixty-fourth is where the head itself moves, which is the step
    /// that makes it unbounded: `a0`, then the sixty-two keys `Zz` down to `Z0`,
    /// and then three characters.
    #[test]
    fn the_head_carries_rather_than_running_out() {
        let mut top = None;

        for _ in 0..64 {
            top = Some(between(None, top.as_deref(), THIS_DEVICE).unwrap());
        }

        assert_eq!(
            key_of(top.as_deref().unwrap()),
            "Yzz",
            "the first key past the two-character ones",
        );
    }

    /// Ranking below everything is the same walk the other way, which is what
    /// the migration's own ranking is made of.
    #[test]
    fn ranking_below_everything_walks_up_the_digits() {
        let mut minted = Vec::new();
        let mut bottom = None;

        for _ in 0..3 {
            let rank = between(bottom.as_deref(), None, THIS_DEVICE).unwrap();

            minted.push(key_of(&rank).to_owned());
            bottom = Some(rank);
        }

        assert_eq!(minted, vec!["a0", "a1", "a2"]);
    }

    /// A rank carries the device that issued it, whoever the neighbours belong
    /// to: that is what makes a drag on a merged list a write to the row's own
    /// device.
    #[test]
    fn a_rank_carries_the_device_that_minted_it() {
        let above = format!("a1{SEPARATOR}{ANOTHER_DEVICE}");
        let below = format!("a2{SEPARATOR}{ANOTHER_DEVICE}");

        let minted = between(Some(&above), Some(&below), THIS_DEVICE).unwrap();

        assert!(
            minted.ends_with(&format!("{SEPARATOR}{THIS_DEVICE}")),
            "{minted} was minted here, so it is this device's",
        );
        assert!(
            above < minted && minted < below,
            "{above} < {minted} < {below} with every suffix on",
        );
    }

    /// Two devices ranking above their own empty list mint the same key, which
    /// is the whole reason for the suffix — and suffixed they sort apart rather
    /// than tying.
    #[test]
    fn two_devices_ranking_above_nothing_sort_apart() {
        let mine = between(None, None, THIS_DEVICE).unwrap();
        let theirs = between(None, None, ANOTHER_DEVICE).unwrap();

        assert_eq!(
            key_of(&mine),
            key_of(&theirs),
            "the same key, computed apart"
        );
        assert_ne!(mine, theirs, "and two ranks all the same");
    }

    /// And nothing sorts between two rows at one key, which is what the suffix
    /// is there to keep from happening: said as a refusal rather than as a rank
    /// that would read differently on two devices.
    #[test]
    fn two_rows_at_one_key_have_nothing_between_them() {
        let mine = format!("a0{SEPARATOR}{THIS_DEVICE}");
        let theirs = format!("a0{SEPARATOR}{ANOTHER_DEVICE}");

        let refused = between(Some(&mine), Some(&theirs), THIS_DEVICE).unwrap_err();

        assert!(
            format!("{refused:#}").contains("the same key"),
            "{refused:#}",
        );
    }

    /// A rank the wrong way round is a refusal rather than a key somewhere else
    /// in the list.
    #[test]
    fn a_pair_the_wrong_way_round_is_refused() {
        let above = format!("a2{SEPARATOR}{THIS_DEVICE}");
        let below = format!("a1{SEPARATOR}{THIS_DEVICE}");

        assert!(between(Some(&above), Some(&below), THIS_DEVICE).is_err());
    }

    /// And a rank this arithmetic cannot read is one too: a hand-edited row is
    /// something to report rather than to rank around.
    #[test]
    fn a_rank_that_is_not_a_key_is_refused() {
        for rank in ["", "!!", "a", "a10", SMALLEST] {
            let rank = format!("{rank}{SEPARATOR}{THIS_DEVICE}");

            assert!(
                between(Some(&rank), None, THIS_DEVICE).is_err(),
                "{rank} is not a key this can work from",
            );
        }
    }

    /// The property the whole scheme stands on, over a few thousand inserts at
    /// random positions: every rank minted is strictly between the two it was
    /// minted between, and the list is still in order afterwards.
    ///
    /// Which is what makes writing the arithmetic here rather than depending on
    /// somebody's safe: the assertion is the definition, and it is made against
    /// every insert rather than against a handful of hand-picked ones.
    #[test]
    fn every_insert_lands_strictly_between_its_neighbours() {
        // A generator rather than the operating system's randomness, so that a
        // failure is a failure somebody can run again. Numerical Recipes'
        // 64-bit xorshift, which is four lines and plenty for picking a
        // position in a list.
        let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
        let mut random = move |bound: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;

            usize::try_from(seed % bound as u64).unwrap()
        };

        let mut list = vec![between(None, None, THIS_DEVICE).unwrap()];

        for insert in 0..4_000 {
            // Every position including the two ends, which are the cases with a
            // neighbour missing: at every start, and at a drag to the top or
            // the bottom.
            let at = random(list.len() + 1);
            let above = (at > 0).then(|| list[at - 1].clone());
            let below = list.get(at).cloned();

            // Half of them from a second device, so the arithmetic is asked to
            // work between suffixes that are not its own as often as not.
            let device = if insert % 2 == 0 {
                THIS_DEVICE
            } else {
                ANOTHER_DEVICE
            };
            let minted = between(above.as_deref(), below.as_deref(), device).unwrap();

            if let Some(above) = &above {
                assert!(above < &minted, "{above} < {minted} at insert {insert}");
            }

            if let Some(below) = &below {
                assert!(&minted < below, "{minted} < {below} at insert {insert}");
            }

            list.insert(at, minted);
        }

        let mut sorted = list.clone();
        sorted.sort();

        assert_eq!(list, sorted, "the list is in rank order after every insert");

        let mut distinct = list.clone();
        distinct.dedup();

        assert_eq!(distinct.len(), list.len(), "and no two rows share a rank");
    }
}
