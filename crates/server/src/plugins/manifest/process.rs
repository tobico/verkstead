//! A `provides:` entry of the `process` Category, read and refused by field —
//! the grammar `docs/plugins.md` writes under *A Process*.
//!
//! **A Process is states over Activities** (ADR-0030, *Process*; Set 1765,
//! Q23): each state runs one of the Activities its plugin or a dependency
//! provides, with options, and its transitions map every Outcome that Activity
//! declares to another state or to one of the three Verkstead owns. Draft is
//! Verkstead's way in and never a target; `steered` is never a row, the steer
//! marks being its mapping; and a `direction:` Outcome is mapped by the
//! Directions the Process offers rather than by a row.
//!
//! **Read in two passes.** [`Walk::process`] reads the shape, which needs
//! nothing but the entry; [`Process::checked`] holds the transitions against
//! the Outcomes of each state's Activity, which needs the Activity found. The
//! manifest reader runs it with what it can know alone — the plugin's own
//! Activities and the built-ins' — and the loader runs it again once the
//! installed dependencies have loaded, so a Process is refused the same way
//! by `verkstead plugin check` and at load (Set 1771, Q1).

use serde_json::{Map, Value};

use super::{
    Activity, Dependency, Pack, Refusal, STEERED, Walk, is_word, skilled, text_of, within,
};
use crate::platform::Platform;

/// The keys a Process's entry holds, its `category` aside.
const KEYS: &[&str] = &[
    "id",
    "name",
    "description",
    "target",
    "roles",
    "directions",
    "states",
    "transitions",
];

const STATE_KEYS: &[&str] = &["id", "word", "activity", "options", "steer", "detour"];

const ROLE_KEYS: &[&str] = &["name", "label", "building"];

/// The Directions Verkstead knows (ADR-0008), in the order the chooser draws
/// them, as a manifest writes them.
pub const DIRECTIONS: [&str; 3] = ["inline", "task-list", "roadmap"];

/// What an Outcome naming a Direction starts with: the interview's
/// `direction:inline`.
const DIRECTION: &str = "direction";

/// The states Verkstead owns. `draft` is where every Conversation starts and
/// nothing returns to; the other three are what a transition may name beside
/// the declared states.
const DRAFT: &str = "draft";
const OWNED: [(&str, To); 3] = [
    ("done", To::Done),
    ("closed", To::Closed),
    ("stopped", To::Stopped),
];

/// The option that names a skill, which is checked as an Activity's skills are.
const SKILL: &str = "skill";

/// A **Process**, as a plugin's manifest declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    /// What the `processes` table names it by, with its plugin's uuid.
    pub id: String,
    /// What the picker draws.
    pub name: String,
    pub description: String,
    /// Whether a Draft of it takes a Target.
    pub target: bool,
    /// Its roles, in the order written, exactly one of them building.
    pub roles: Vec<Role>,
    /// The Directions its interview offers, each with the state it leads to,
    /// in the order of [`DIRECTIONS`].
    pub directions: Vec<(String, String)>,
    /// Its states in order: the first is what Start enters.
    pub states: Vec<State>,
    /// Its entry's whole name, `provides[2]`, for the refusals the loader
    /// meets once the dependencies have loaded.
    pub(crate) at: String,
}

/// One of a Process's roles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    /// What its Pairing is stored by: `implementation`, or one of the plugin's.
    pub name: String,
    pub label: String,
    /// Whether it is the role the Done signal, memos and previews assume.
    pub building: bool,
}

/// One of a Process's states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// The word the `conversations.state` column holds.
    pub id: String,
    /// What the Conversation card draws.
    pub word: String,
    pub activity: Runs,
    /// What the Activity is started with, beside the Conversation's facts.
    pub options: Vec<(String, String)>,
    /// The brief a steer into it takes, where it takes one.
    pub steer: Option<Brief>,
    /// Whether it returns to the state it was steered from, whatever its
    /// Activity ends in.
    pub detour: bool,
    /// Each Outcome of its Activity, and where it leads. Empty for a detour.
    pub transitions: Vec<(String, To)>,
}

/// Which Activity a state runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Runs {
    /// The one Activity, by its id.
    Activity(String),
    /// One per Direction the Process offers, the Conversation's stored
    /// Direction picking (Set 1879, Q8): each Direction and the Activity's id.
    ByDirection(Vec<(String, String)>),
}

impl Runs {
    /// Each Activity it may run, with the Direction it is run for where it
    /// is one of several.
    pub fn activities(&self) -> Vec<(Option<&str>, &str)> {
        match self {
            Runs::Activity(id) => vec![(None, id.as_str())],
            Runs::ByDirection(each) => each
                .iter()
                .map(|(direction, id)| (Some(direction.as_str()), id.as_str()))
                .collect(),
        }
    }
}

/// The brief a steer into a state takes: each a kind Verkstead knows how to
/// draw on the Steer form and hand to the Activity (Set 1877, Q5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Brief {
    /// A new round's Brief, and how much of the last interview to carry.
    Round,
    /// What the human wants done.
    Instruction,
    /// What to take up about work already pushed.
    FollowUp,
    /// The question an investigation answers.
    Question,
    /// Nothing: the steer takes no brief.
    None,
}

impl Brief {
    const ALL: [(&'static str, Brief); 5] = [
        ("round", Brief::Round),
        ("instruction", Brief::Instruction),
        ("follow-up", Brief::FollowUp),
        ("question", Brief::Question),
        ("none", Brief::None),
    ];

    /// The word a manifest writes.
    pub fn word(self) -> &'static str {
        Brief::ALL
            .iter()
            .find(|(_, brief)| *brief == self)
            .map(|(word, _)| *word)
            .expect("every brief has its word")
    }
}

/// Where an Outcome leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum To {
    /// A declared state, by its id.
    State(String),
    Done,
    Closed,
    /// The Conversation stays where it stands, Stopped, with the Activity's
    /// stop Notice, waiting on Resume.
    Stopped,
}

/// What looking a state's Activity up came to.
#[derive(Debug, Clone)]
pub enum Found {
    /// The plugin or a dependency provides it.
    Known(Activity),
    /// A dependency that isn't loaded yet might: the loader looks again.
    Unknown,
    /// Neither the plugin nor any dependency does.
    Unreachable,
}

impl Process {
    /// The role the Done signal, memos and previews assume.
    pub fn building(&self) -> &Role {
        self.roles
            .iter()
            .find(|role| role.building)
            .expect("a Process is read with exactly one building role")
    }

    /// The state `id`, where it is declared.
    pub fn state(&self, id: &str) -> Option<&State> {
        self.states.iter().find(|state| state.id == id)
    }

    /// Refused where a state runs an Activity `find` says is unreachable, or
    /// where an Outcome of one `find` knows is left unmapped, or a row maps an
    /// Outcome it never ends in.
    pub fn checked(&self, find: impl Fn(&str) -> Found) -> Result<(), Refusal> {
        let at = &self.at;
        for (index, state) in self.states.iter().enumerate() {
            let mut known: Vec<Activity> = Vec::new();
            let mut all_known = true;
            for (direction, id) in state.activity.activities() {
                let field = match direction {
                    Some(direction) => format!("{at}.states[{index}].activity.{direction}"),
                    None => format!("{at}.states[{index}].activity"),
                };
                match find(id) {
                    Found::Known(activity) => known.push(activity),
                    Found::Unknown => all_known = false,
                    Found::Unreachable => {
                        return Err(Refusal::of(
                            &field,
                            format!(
                                "`{field}` is `{id}`, an Activity neither this plugin nor one it \
                                 depends on provides. Add a dependency on the plugin that \
                                 provides it."
                            ),
                        ));
                    }
                }
            }

            // A detour's Outcomes all lead back where it was steered from.
            if state.detour {
                continue;
            }

            let row = format!("{at}.transitions.{}", state.id);
            for activity in &known {
                for outcome in &activity.outcomes {
                    if outcome
                        .split_once(':')
                        .is_some_and(|(of, _)| of == DIRECTION)
                    {
                        if self.directions.is_empty() {
                            let field = format!("{at}.directions");
                            return Err(Refusal::of(
                                &field,
                                format!(
                                    "`{field}` is missing. The state `{}` runs {}, which ends \
                                     in a Direction: offer at least one.",
                                    state.id, activity.name
                                ),
                            ));
                        }
                        continue;
                    }
                    if !state
                        .transitions
                        .iter()
                        .any(|(mapped, _)| mapped == outcome)
                    {
                        return Err(Refusal::of(
                            &row,
                            format!(
                                "`{row}` leaves the Outcome `{outcome}` unmapped: the state `{}` \
                                 runs {}, which can end in it. Map it to a state, or to \
                                 `done`, `closed` or `stopped`.",
                                state.id, activity.name
                            ),
                        ));
                    }
                }
            }

            if all_known {
                for (outcome, _) in &state.transitions {
                    if !known
                        .iter()
                        .any(|activity| activity.outcomes.contains(outcome))
                    {
                        let field = format!("{row}.{outcome}");
                        let names: Vec<&str> = known
                            .iter()
                            .map(|activity| activity.name.as_str())
                            .collect();
                        return Err(Refusal::of(
                            &field,
                            format!(
                                "`{field}` maps `{outcome}`, and the state `{}` runs {}, which \
                                 never ends in it.",
                                state.id,
                                names.join(" or ")
                            ),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

impl Walk<'_> {
    /// The `process` entry `body`, at `field`, read for its shape. Each skill
    /// a state's options name is checked against `packs` and `dependencies`
    /// as an Activity's skills are.
    pub(super) fn process(
        &self,
        body: &Map<String, Value>,
        field: &str,
        packs: &[Pack],
        dependencies: &[Dependency],
        platform: Platform,
    ) -> Result<Process, Refusal> {
        if let Some(key) = body.keys().find(|key| !KEYS.contains(&key.as_str())) {
            return Err(Refusal::unknown(&format!("{field}.{key}")));
        }

        let id_field = format!("{field}.id");
        let id = text_of(within(body, field, "id")?, &id_field)?;
        if !is_word(&id) {
            return Err(Refusal::of(
                &id_field,
                format!(
                    "`{id_field}` should be lowercase letters, digits and hyphens, like \
                     `design-rounds`."
                ),
            ));
        }
        let name = text_of(within(body, field, "name")?, &format!("{field}.name"))?;
        let description = text_of(
            within(body, field, "description")?,
            &format!("{field}.description"),
        )?;
        let target = flag(body, field, "target")?;
        let roles = roles(body, field)?;

        let mut states = states(body, field, packs, dependencies, platform)?;
        let directions = directions(body, field, &states)?;
        for (index, state) in states.iter().enumerate() {
            let Runs::ByDirection(each) = &state.activity else {
                continue;
            };
            let at = format!("{field}.states[{index}].activity");
            for (direction, _) in each {
                if !directions.iter().any(|(offered, _)| offered == direction) {
                    let field = format!("{at}.{direction}");
                    return Err(Refusal::of(
                        &field,
                        format!(
                            "`{field}` names an Activity for `{direction}`, which the Process \
                             doesn't offer in `directions`."
                        ),
                    ));
                }
            }
            for (offered, _) in &directions {
                if !each.iter().any(|(direction, _)| direction == offered) {
                    let field = format!("{at}.{offered}");
                    return Err(Refusal::of(
                        &field,
                        format!(
                            "`{field}` is missing. The Process offers `{offered}`, so the state \
                             needs an Activity for it."
                        ),
                    ));
                }
            }
        }
        transitions(body, field, &mut states)?;

        Ok(Process {
            id,
            name,
            description,
            target,
            roles,
            directions,
            states,
            at: field.to_owned(),
        })
    }
}

/// The yes-or-no at `key` of `map`; absent is no.
fn flag(map: &Map<String, Value>, field: &str, key: &str) -> Result<bool, Refusal> {
    match map.get(key) {
        Some(Value::Null) | None => Ok(false),
        Some(Value::Bool(flag)) => Ok(*flag),
        Some(_) => {
            let at = format!("{field}.{key}");
            Err(Refusal::of(
                &at,
                format!("`{at}` should be `true` or `false`."),
            ))
        }
    }
}

/// The map at `key`, named `at`; absent is empty.
fn map_at<'a>(
    map: &'a Map<String, Value>,
    key: &str,
    at: &str,
) -> Result<Option<&'a Map<String, Value>>, Refusal> {
    match map.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(Value::Object(inner)) => Ok(Some(inner)),
        Some(_) => Err(Refusal::of(at, format!("`{at}` should be a map."))),
    }
}

/// The non-empty list at `key`, named `at`.
fn listed<'a>(map: &'a Map<String, Value>, field: &str, key: &str) -> Result<&'a [Value], Refusal> {
    let at = format!("{field}.{key}");
    match within(map, field, key)? {
        Value::Array(entries) if entries.is_empty() => {
            Err(Refusal::of(&at, format!("`{at}` is empty.")))
        }
        Value::Array(entries) => Ok(entries),
        _ => Err(Refusal::of(&at, format!("`{at}` should be a list."))),
    }
}

/// A map whose keys are all `allowed`, at `at`.
fn shaped<'a>(
    value: &'a Value,
    at: &str,
    allowed: &[&str],
) -> Result<&'a Map<String, Value>, Refusal> {
    let Value::Object(map) = value else {
        return Err(Refusal::of(at, format!("`{at}` should be a map.")));
    };
    if let Some(key) = map.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(Refusal::unknown(&format!("{at}.{key}")));
    }
    Ok(map)
}

fn roles(body: &Map<String, Value>, field: &str) -> Result<Vec<Role>, Refusal> {
    let mut roles: Vec<Role> = Vec::new();
    for (index, value) in listed(body, field, "roles")?.iter().enumerate() {
        let at = format!("{field}.roles[{index}]");
        let map = shaped(value, &at, ROLE_KEYS)?;
        let name_field = format!("{at}.name");
        let name = text_of(within(map, &at, "name")?, &name_field)?;
        if !is_word(&name) {
            return Err(Refusal::of(
                &name_field,
                format!(
                    "`{name_field}` should be lowercase letters, digits and hyphens, like \
                     `implementation`."
                ),
            ));
        }
        if roles.iter().any(|seen| seen.name == name) {
            return Err(Refusal::of(
                &name_field,
                format!("`{name_field}` is `{name}`, which an earlier role already uses."),
            ));
        }
        let label = text_of(within(map, &at, "label")?, &format!("{at}.label"))?;
        let building = flag(map, &at, "building")?;
        if building && roles.iter().any(|seen| seen.building) {
            let field = format!("{at}.building");
            return Err(Refusal::of(
                &field,
                format!(
                    "`{field}` marks a second building role. Exactly one role is the building \
                     role."
                ),
            ));
        }
        roles.push(Role {
            name,
            label,
            building,
        });
    }
    if !roles.iter().any(|role| role.building) {
        let field = format!("{field}.roles");
        return Err(Refusal::of(
            &field,
            format!(
                "`{field}` has no building role. Mark the one the Done signal, memos and \
                 previews assume with `building: true`."
            ),
        ));
    }
    Ok(roles)
}

/// Whether `text` may be a state's id: lowercase letters, digits, hyphens and
/// underscores — the words already stored are `follow_up` and the like.
fn is_state(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
}

fn states(
    body: &Map<String, Value>,
    field: &str,
    packs: &[Pack],
    dependencies: &[Dependency],
    platform: Platform,
) -> Result<Vec<State>, Refusal> {
    let mut states: Vec<State> = Vec::new();
    for (index, value) in listed(body, field, "states")?.iter().enumerate() {
        let at = format!("{field}.states[{index}]");
        let map = shaped(value, &at, STATE_KEYS)?;

        let id_field = format!("{at}.id");
        let id = text_of(within(map, &at, "id")?, &id_field)?;
        if !is_state(&id) {
            return Err(Refusal::of(
                &id_field,
                format!(
                    "`{id_field}` should be lowercase letters, digits, hyphens and underscores, \
                     like `follow_up`."
                ),
            ));
        }
        if id == DRAFT || OWNED.iter().any(|(owned, _)| *owned == id) {
            return Err(Refusal::of(
                &id_field,
                format!(
                    "`{id_field}` is `{id}`, which is Verkstead's own state. Give the state a \
                     word of its own."
                ),
            ));
        }
        if states.iter().any(|seen| seen.id == id) {
            return Err(Refusal::of(
                &id_field,
                format!("`{id_field}` is `{id}`, which an earlier state already uses."),
            ));
        }

        let word = text_of(within(map, &at, "word")?, &format!("{at}.word"))?;
        let activity = runs(within(map, &at, "activity")?, &format!("{at}.activity"))?;
        let options = options(map, &at, packs, dependencies, platform)?;

        let steer = match map.get("steer") {
            Some(Value::Null) | None => None,
            Some(value) => {
                let steer_field = format!("{at}.steer");
                let steer = shaped(value, &steer_field, &["brief"])?;
                let brief_field = format!("{steer_field}.brief");
                let written = text_of(within(steer, &steer_field, "brief")?, &brief_field)?;
                let Some((_, brief)) = Brief::ALL.iter().find(|(word, _)| *word == written) else {
                    return Err(Refusal::of(
                        &brief_field,
                        format!(
                            "`{brief_field}` is `{written}`, which isn't a brief Verkstead \
                             knows: `round`, `instruction`, `follow-up`, `question` or `none`."
                        ),
                    ));
                };
                Some(*brief)
            }
        };
        let detour = flag(map, &at, "detour")?;

        states.push(State {
            id,
            word,
            activity,
            options,
            steer,
            detour,
            transitions: Vec::new(),
        });
    }
    Ok(states)
}

/// A state's `activity`: an Activity's id, or a map of Direction to one.
fn runs(value: &Value, at: &str) -> Result<Runs, Refusal> {
    let activity = |value: &Value, at: &str| {
        let id = text_of(value, at)?;
        if is_word(&id) {
            Ok(id)
        } else {
            Err(Refusal::of(
                at,
                format!(
                    "`{at}` should be an Activity's id, lowercase letters, digits and hyphens, \
                     like `follow-up-rounds`."
                ),
            ))
        }
    };

    let Value::Object(each) = value else {
        return activity(value, at).map(Runs::Activity);
    };
    if each.is_empty() {
        return Err(Refusal::of(at, format!("`{at}` is empty.")));
    }
    if let Some(key) = each.keys().find(|key| !DIRECTIONS.contains(&key.as_str())) {
        let field = format!("{at}.{key}");
        return Err(Refusal::of(
            &field,
            format!(
                "`{field}` isn't a Direction. An Activity per Direction is keyed by `inline`, \
                 `task-list` or `roadmap`."
            ),
        ));
    }
    DIRECTIONS
        .iter()
        .filter_map(|direction| Some((*direction, each.get(*direction)?)))
        .map(|(direction, value)| {
            Ok((
                direction.to_owned(),
                activity(value, &format!("{at}.{direction}"))?,
            ))
        })
        .collect::<Result<_, _>>()
        .map(Runs::ByDirection)
}

/// A state's `options`, each key's text. The `skill` option is checked as an
/// Activity's skills are.
fn options(
    map: &Map<String, Value>,
    at: &str,
    packs: &[Pack],
    dependencies: &[Dependency],
    platform: Platform,
) -> Result<Vec<(String, String)>, Refusal> {
    let field = format!("{at}.options");
    let Some(options) = map_at(map, "options", &field)? else {
        return Ok(Vec::new());
    };
    let mut read = Vec::new();
    for (key, value) in options {
        let at = format!("{field}.{key}");
        let text = match value {
            Value::String(_) => text_of(value, &at)?,
            Value::Bool(flag) => flag.to_string(),
            Value::Number(number) => number.to_string(),
            _ => {
                return Err(Refusal::of(
                    &at,
                    format!("`{at}` should be text, a number, or `true` or `false`."),
                ));
            }
        };
        if key == SKILL {
            skilled(&text, &at, packs, dependencies, platform)?;
        }
        read.push((key.clone(), text));
    }
    Ok(read)
}

/// The Directions offered, each with the state it leads to, in the order of
/// [`DIRECTIONS`].
fn directions(
    body: &Map<String, Value>,
    field: &str,
    states: &[State],
) -> Result<Vec<(String, String)>, Refusal> {
    let at = format!("{field}.directions");
    let Some(offered) = map_at(body, "directions", &at)? else {
        return Ok(Vec::new());
    };
    if let Some(key) = offered
        .keys()
        .find(|key| !DIRECTIONS.contains(&key.as_str()))
    {
        let field = format!("{at}.{key}");
        return Err(Refusal::of(
            &field,
            format!("`{field}` isn't a Direction: `inline`, `task-list` or `roadmap`."),
        ));
    }
    DIRECTIONS
        .iter()
        .filter_map(|direction| Some((*direction, offered.get(*direction)?)))
        .map(|(direction, value)| {
            let field = format!("{at}.{direction}");
            let state = text_of(value, &field)?;
            if !states.iter().any(|declared| declared.id == state) {
                return Err(Refusal::of(
                    &field,
                    format!("`{field}` is `{state}`, which isn't one of the Process's states."),
                ));
            }
            Ok((direction.to_owned(), state))
        })
        .collect()
}

/// `transitions`, read onto the states they leave.
fn transitions(
    body: &Map<String, Value>,
    field: &str,
    states: &mut [State],
) -> Result<(), Refusal> {
    let at = format!("{field}.transitions");
    let Some(rows) = map_at(body, "transitions", &at)? else {
        return Ok(());
    };
    let declared: Vec<String> = states.iter().map(|state| state.id.clone()).collect();

    for (from, row) in rows {
        let row_field = format!("{at}.{from}");
        let Some(state) = states.iter_mut().find(|state| state.id == *from) else {
            return Err(Refusal::of(
                &row_field,
                format!("`{row_field}` leaves `{from}`, which isn't one of the Process's states."),
            ));
        };
        if state.detour {
            return Err(Refusal::of(
                &row_field,
                format!(
                    "`{row_field}` maps a detour's Outcomes, and a detour returns to the state it \
                     was steered from whatever it ends in. Take the row out."
                ),
            ));
        }
        let Value::Object(row) = row else {
            return Err(Refusal::of(
                &row_field,
                format!("`{row_field}` should be a map of each Outcome to where it leads."),
            ));
        };
        for (outcome, to) in row {
            let to_field = format!("{row_field}.{outcome}");
            if outcome == STEERED {
                return Err(Refusal::of(
                    &to_field,
                    format!(
                        "`{to_field}` maps `{STEERED}`, which a row never does: a state's \
                         `steer` mark is where a steer leads. Take it out."
                    ),
                ));
            }
            if outcome
                .split_once(':')
                .is_some_and(|(of, _)| of == DIRECTION)
            {
                return Err(Refusal::of(
                    &to_field,
                    format!(
                        "`{to_field}` maps a Direction, which the Process's `directions` does. \
                         Take it out."
                    ),
                ));
            }
            let written = text_of(to, &to_field)?;
            let to = if declared.contains(&written) {
                To::State(written)
            } else if let Some((_, owned)) = OWNED.iter().find(|(owned, _)| *owned == written) {
                owned.clone()
            } else if written == DRAFT {
                return Err(Refusal::of(
                    &to_field,
                    format!(
                        "`{to_field}` is `{DRAFT}`, which nothing returns to. Map it to a state, \
                         or to `done`, `closed` or `stopped`."
                    ),
                ));
            } else {
                return Err(Refusal::of(
                    &to_field,
                    format!(
                        "`{to_field}` is `{written}`, which isn't one of the Process's states, \
                         nor `done`, `closed` or `stopped`."
                    ),
                ));
            };
            state.transitions.push((outcome.clone(), to));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{FILE, Manifest};
    use super::*;
    use crate::plugins::built_in::{ACTIVITIES, VERKSTEAD};

    /// A plugin of an Activity of its own, `rounds`, ending in `committed` or
    /// `nothing`, and the one Process `process`, depending on `dependencies`.
    fn processing(process: &str, dependencies: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("rounds.wasm"), b"\0asm").unwrap();
        std::fs::write(
            dir.path().join(FILE),
            format!(
                "uuid: 0b6f6b7e-5c3a-4d0e-9a51-2f7d8c1e4a90\nname: Rounds\nauthor:\n  \
                 name: Ada Lovelace\n  email: ada@example.com\nversion: 1.0.0\ninterface: 1\n\
                 provides:\n  - category: activity\n    id: rounds\n    name: Rounds\n    \
                 component: rounds.wasm\n    outcomes: [committed, nothing]\n\
                 {process}{dependencies}"
            ),
        )
        .unwrap();
        dir
    }

    /// A dependency on the built-in `uuid`, called `name`.
    fn on(uuid: &str, name: &str) -> String {
        format!(
            "  - uuid: {uuid}\n    name: {name}\n    author:\n      name: Tobias Cohen\n      \
             email: tobi@tobico.net\n    version: ^0.1\n"
        )
    }

    fn on_the_built_ins() -> String {
        format!(
            "dependencies:\n{}{}",
            on(ACTIVITIES, "Activities"),
            on(VERKSTEAD, "Verkstead")
        )
    }

    /// A Process of two states: `rounds` over the plugin's own Activity, then
    /// `wrapping` over the built-in Wrap-up, with `rest` after its states.
    fn tinkering(rest: &str) -> String {
        format!(
            "  - category: process\n    id: tinker\n    name: Tinker\n    \
             description: Rounds, then the wrap-up.\n    roles:\n      \
             - {{name: implementation, label: Implementation, building: true}}\n      \
             - {{name: review, label: Review}}\n    states:\n      \
             - id: follow_up\n        word: Following up\n        activity: rounds\n        \
             options: {{skill: verkstead/following-up}}\n        steer: {{brief: follow-up}}\n      \
             - id: wrapping\n        word: Wrapping up\n        activity: wrap-up\n{rest}"
        )
    }

    const MAPPED: &str = "    transitions:\n      follow_up: {committed: wrapping, nothing: done}\n      \
                          wrapping: {settled: done, merged: done, closed: stopped, given-up: stopped}\n";

    fn read(dir: &tempfile::TempDir) -> Result<Manifest, Refusal> {
        Manifest::read(dir.path(), Platform::Linux)
    }

    fn refused(process: &str, dependencies: &str) -> Refusal {
        read(&processing(process, dependencies)).expect_err("the Process should be refused")
    }

    #[test]
    fn a_complete_process_is_read() {
        let manifest = read(&processing(&tinkering(MAPPED), &on_the_built_ins())).unwrap();

        let [process] = manifest.processes.as_slice() else {
            panic!("one Process: {:?}", manifest.processes);
        };
        assert_eq!(process.id, "tinker");
        assert_eq!(process.name, "Tinker");
        assert!(!process.target);
        assert_eq!(process.building().name, "implementation");
        let [follow_up, wrapping] = process.states.as_slice() else {
            panic!("two states: {:?}", process.states);
        };
        assert_eq!(follow_up.id, "follow_up");
        assert_eq!(follow_up.word, "Following up");
        assert_eq!(follow_up.activity, Runs::Activity("rounds".to_owned()));
        assert_eq!(
            follow_up.options,
            [("skill".to_owned(), "verkstead/following-up".to_owned())]
        );
        assert_eq!(follow_up.steer, Some(Brief::FollowUp));
        assert!(!follow_up.detour);
        assert!(
            follow_up
                .transitions
                .contains(&("committed".to_owned(), To::State("wrapping".to_owned())))
        );
        assert!(
            wrapping
                .transitions
                .contains(&("given-up".to_owned(), To::Stopped))
        );
        assert_eq!(wrapping.steer, None);
    }

    #[test]
    fn an_outcome_left_unmapped_is_refused_naming_the_state_and_the_outcome() {
        let refusal = refused(
            &tinkering(&MAPPED.replace(", given-up: stopped", "")),
            &on_the_built_ins(),
        );

        assert_eq!(
            refusal.field.as_deref(),
            Some("provides[1].transitions.wrapping")
        );
        assert!(
            refusal.why.contains("`given-up` unmapped") && refusal.why.contains("`wrapping`"),
            "{refusal}"
        );
    }

    #[test]
    fn a_state_with_no_row_at_all_is_refused_for_its_first_outcome() {
        let refusal = refused(
            &tinkering("    transitions:\n      follow_up: {committed: wrapping, nothing: done}\n"),
            &on_the_built_ins(),
        );

        assert!(refusal.why.contains("`settled` unmapped"), "{refusal}");
    }

    #[test]
    fn an_activity_from_a_plugin_not_depended_on_is_refused_naming_it() {
        let refusal = refused(
            &tinkering(MAPPED),
            &format!("dependencies:\n{}", on(VERKSTEAD, "Verkstead")),
        );

        assert_eq!(
            refusal.field.as_deref(),
            Some("provides[1].states[1].activity")
        );
        assert!(
            refusal.why.contains("`wrap-up`") && refusal.why.contains("Add a dependency"),
            "{refusal}"
        );
    }

    #[test]
    fn a_skill_from_a_plugin_not_depended_on_is_refused_naming_it() {
        let refusal = refused(
            &tinkering(MAPPED),
            &format!("dependencies:\n{}", on(ACTIVITIES, "Activities")),
        );

        assert_eq!(
            refusal.field.as_deref(),
            Some("provides[1].states[0].options.skill")
        );
        assert!(
            refusal.why.contains("`verkstead/following-up`"),
            "{refusal}"
        );
    }

    #[test]
    fn an_activity_an_installed_dependency_may_provide_is_left_to_the_loader() {
        let installed = "dependencies:\n  - uuid: 7c1d2a43-0f5e-4b9a-8c6d-3e2f1a0b9c8d\n    \
                         name: Shared\n    author:\n      name: Grace Hopper\n      \
                         email: grace@example.com\n    version: ^2\n";
        let process = tinkering(MAPPED).replace("activity: wrap-up", "activity: watching");

        let manifest = read(&processing(
            &process.replace("options: {skill: verkstead/following-up}\n        ", ""),
            installed,
        ))
        .unwrap();

        assert_eq!(
            manifest.processes[0].states[1].activity,
            Runs::Activity("watching".to_owned())
        );
    }

    #[test]
    fn a_row_mapping_an_outcome_the_activity_never_ends_in_is_refused() {
        let refusal = refused(
            &tinkering(&MAPPED.replace("nothing: done", "nothing: done, abandoned: done")),
            &on_the_built_ins(),
        );

        assert_eq!(
            refusal.field.as_deref(),
            Some("provides[1].transitions.follow_up.abandoned")
        );
    }

    #[test]
    fn a_process_needs_exactly_one_building_role() {
        let none = refused(
            &tinkering(MAPPED).replace(", building: true", ""),
            &on_the_built_ins(),
        );
        assert_eq!(none.field.as_deref(), Some("provides[1].roles"));

        let two = refused(
            &tinkering(MAPPED).replace("label: Review}", "label: Review, building: true}"),
            &on_the_built_ins(),
        );
        assert_eq!(two.field.as_deref(), Some("provides[1].roles[1].building"));
    }

    #[test]
    fn a_transition_names_only_declared_states_and_verksteads_own() {
        let to_draft = refused(
            &tinkering(&MAPPED.replace("nothing: done", "nothing: draft")),
            &on_the_built_ins(),
        );
        assert_eq!(
            to_draft.field.as_deref(),
            Some("provides[1].transitions.follow_up.nothing")
        );
        assert!(to_draft.why.contains("nothing returns to"), "{to_draft}");

        let to_nowhere = refused(
            &tinkering(&MAPPED.replace("nothing: done", "nothing: elsewhere")),
            &on_the_built_ins(),
        );
        assert!(to_nowhere.why.contains("`elsewhere`"), "{to_nowhere}");

        let from_nowhere = refused(
            &tinkering(&format!("{MAPPED}      elsewhere: {{committed: done}}\n")),
            &on_the_built_ins(),
        );
        assert_eq!(
            from_nowhere.field.as_deref(),
            Some("provides[1].transitions.elsewhere")
        );

        let steered = refused(
            &tinkering(&MAPPED.replace("nothing: done", "nothing: done, steered: done")),
            &on_the_built_ins(),
        );
        assert!(steered.why.contains("`steer` mark"), "{steered}");
    }

    #[test]
    fn a_state_is_not_one_of_verksteads_own() {
        let refusal = refused(
            &tinkering(MAPPED).replace("id: wrapping", "id: done"),
            &on_the_built_ins(),
        );

        assert_eq!(refusal.field.as_deref(), Some("provides[1].states[1].id"));
    }

    /// Develop's shape: an interview offering Directions, then a state whose
    /// Activity is the Direction's.
    fn developing(directions: &str, implementing: &str) -> String {
        format!(
            "  - category: process\n    id: develop\n    name: Develop\n    \
             description: The ladder.\n    roles:\n      \
             - {{name: implementation, label: Implementation, building: true}}\n    \
             directions: {directions}\n    states:\n      \
             - {{id: grilling, word: Grilling, activity: interview, steer: {{brief: round}}}}\n      \
             - id: implementing\n        word: Implementing\n        activity: {implementing}\n      \
             - {{id: investigating, word: Investigating, activity: rounds, detour: true, \
             options: {{skill: verkstead/investigating}}, steer: {{brief: question}}}}\n    \
             transitions:\n      grilling: {{gone: stopped}}\n      \
             implementing: {{done: done, refused: stopped, finished: done, continued: done}}\n"
        )
    }

    const ALL_THREE: &str =
        "{inline: implementing, task-list: implementing, roadmap: implementing}";
    const PER_DIRECTION: &str =
        "{inline: one-session, task-list: backlog-runner, roadmap: stage-runner}";

    #[test]
    fn a_state_may_run_an_activity_per_direction() {
        let manifest = read(&processing(
            &developing(ALL_THREE, PER_DIRECTION),
            &on_the_built_ins(),
        ))
        .unwrap();

        let process = &manifest.processes[0];
        assert_eq!(
            process.directions,
            [
                ("inline".to_owned(), "implementing".to_owned()),
                ("task-list".to_owned(), "implementing".to_owned()),
                ("roadmap".to_owned(), "implementing".to_owned()),
            ]
        );
        assert_eq!(
            process.states[1].activity,
            Runs::ByDirection(vec![
                ("inline".to_owned(), "one-session".to_owned()),
                ("task-list".to_owned(), "backlog-runner".to_owned()),
                ("roadmap".to_owned(), "stage-runner".to_owned()),
            ])
        );
        assert!(process.states[2].detour);
        assert_eq!(process.states[2].steer, Some(Brief::Question));
    }

    #[test]
    fn an_outcome_of_any_of_a_states_activities_left_unmapped_is_refused() {
        let refusal = refused(
            &developing(ALL_THREE, PER_DIRECTION).replace(", continued: done", ""),
            &on_the_built_ins(),
        );

        assert!(refusal.why.contains("`continued` unmapped"), "{refusal}");
    }

    #[test]
    fn a_direction_mapped_to_no_state_is_refused() {
        let refusal = refused(
            &developing(
                "{inline: implementing, task-list: building}",
                "{inline: one-session, task-list: backlog-runner}",
            )
            .replace(", continued: done", ""),
            &on_the_built_ins(),
        );

        assert_eq!(
            refusal.field.as_deref(),
            Some("provides[1].directions.task-list")
        );
    }

    #[test]
    fn a_direction_offered_needs_its_activity_on_each_state_run_per_direction() {
        let refusal = refused(
            &developing(
                ALL_THREE,
                "{inline: one-session, task-list: backlog-runner}",
            ),
            &on_the_built_ins(),
        );

        assert_eq!(
            refusal.field.as_deref(),
            Some("provides[1].states[1].activity.roadmap")
        );
    }

    #[test]
    fn an_interview_with_no_directions_offered_is_refused() {
        let refusal = refused(
            &developing("{}", "one-session").replace(", finished: done, continued: done", ""),
            &on_the_built_ins(),
        );

        assert_eq!(refusal.field.as_deref(), Some("provides[1].directions"));
    }

    #[test]
    fn a_detour_takes_no_transitions() {
        let refusal = refused(
            &format!(
                "{}      investigating: {{committed: done, nothing: done}}\n",
                developing(ALL_THREE, PER_DIRECTION)
            ),
            &on_the_built_ins(),
        );

        assert_eq!(
            refusal.field.as_deref(),
            Some("provides[1].transitions.investigating")
        );
    }

    #[test]
    fn a_brief_verkstead_does_not_know_is_refused() {
        let refusal = refused(
            &tinkering(MAPPED).replace("brief: follow-up", "brief: memo"),
            &on_the_built_ins(),
        );

        assert_eq!(
            refusal.field.as_deref(),
            Some("provides[1].states[0].steer.brief")
        );
    }

    #[test]
    fn an_unknown_key_in_a_process_or_a_state_is_refused() {
        let at_top = refused(
            &tinkering(MAPPED).replace("    roles:", "    colour: red\n    roles:"),
            &on_the_built_ins(),
        );
        assert_eq!(at_top.field.as_deref(), Some("provides[1].colour"));

        let in_a_state = refused(
            &tinkering(MAPPED).replace("word: Wrapping up", "word: Wrapping up\n        wait: 5"),
            &on_the_built_ins(),
        );
        assert_eq!(
            in_a_state.field.as_deref(),
            Some("provides[1].states[1].wait")
        );
    }
}
