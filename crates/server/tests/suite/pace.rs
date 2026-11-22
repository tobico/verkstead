//! How much longer than its own clock a suite here is allowed to take.
//!
//! One module rather than a copy per suite, because what it reads is one
//! number about one machine: a second ruler parsing the same variable would be
//! a second answer to *how loaded is this runner*, and two suites disagreeing
//! about that is the flake all over again.
//!
//! Not every suite needs it. The ones that do are the ones judged on wall
//! clock — a real server up, a real session in a real sandbox, a Nudge crossing
//! a real link — where a machine running at half speed deschedules something
//! past a budget and the test fails about the machine rather than about the
//! code.

use std::sync::LazyLock;
use std::time::Duration;

/// The factor, read once from `VERKSTEAD_TEST_PACE`.
///
/// Unset is `1.0`, which is what a developer's own machine runs at; CI sets
/// `2`, because a shared four-core runner, four tests at a time and once with
/// the workspace building alongside, is the loaded shape these flakes came
/// from.
///
/// **Uniformly, and that is not a detail.** A suite's budgets are chosen
/// against each other — one window deliberately inside another, so that a
/// thing which happened early and a thing which never happened read
/// differently — and scaling them by different factors would quietly delete
/// the ordering several tests exist to prove. One multiplier over all of them
/// keeps every such ordering as it was written.
///
/// A server's own `Pace::default` is untouched: this is a test harness
/// deciding how long to wait, not Verkstead deciding how fast to work.
static PACE: LazyLock<f64> = LazyLock::new(|| {
    let Ok(factor) = std::env::var("VERKSTEAD_TEST_PACE") else {
        return 1.0;
    };

    factor
        .parse::<f64>()
        .ok()
        .filter(|factor| factor.is_finite() && *factor > 0.0)
        .unwrap_or_else(|| panic!("VERKSTEAD_TEST_PACE is a positive number, not {factor:?}"))
});

/// A span of a suite's own, stretched by [`PACE`].
///
/// Everything a suite writes as a literal duration goes through here. Anything
/// derived from a `Pace` the server was handed does not, and must not: the Pace
/// it came from was paced already, and pacing it twice would stretch the window
/// past the ordering it was chosen against.
pub(crate) fn paced(span: Duration) -> Duration {
    span.mul_f64(*PACE)
}

/// Wait out a span of a suite's own — see [`paced`].
///
/// Two shapes use one: the gap between two reads in a loop that is waiting for
/// something to happen, and the do-nothing window a negative assertion holds
/// open. Both stretch for the same reason. A window has to stay long against
/// whatever else is running, or it stops being evidence that nothing happened;
/// and a loop that reads less often on a loaded machine is a loop taking less
/// of the machine the thing it is waiting for needs.
pub(crate) async fn pause(span: Duration) {
    tokio::time::sleep(paced(span)).await;
}
