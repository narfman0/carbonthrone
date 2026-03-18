`TemporalFlux` resource, `PendingEffects`, and all five temporal `AbilityEffect` variants are in `crates/core/src/`. Five Researcher abilities (Temporal Recall, Temporal Displacement, Acceleration, Entropic Rounds, Echo Strike) are live. Flux bar shown at top-center of combat UI.

## The Framing: "Temporal Technology"

In a sci-fi context, chronurgy doesn't need to feel like magic. It could be:

* **Quantum field manipulation** — collapsing or expanding probability states
* **Localized spacetime distortion** — military-grade tech derived from FTL research
* **Entropic weaponry** — accelerating or reversing the thermodynamic arrow of time

This keeps it grounded while justifying wild effects.

---

## Weapon & Ability Categories

**Temporal Displacement (Offensive)**
A round or pulse that hits a target and *delays* the damage. The enemy takes the hit on your *next* turn instead — right when they might have moved to safety, or while a teammate is attacking. The psychological layer is interesting: enemies under a delay debuff are running from damage that hasn't landed yet.

**Rewind (Defensive / Utility)**
A unit can *revert* to the state it was in 1-2 turns ago — position, HP, status effects. Costs a heavy resource. Doesn't undo *other units'* actions, so the battlefield has changed around them. This is powerful but disorienting — you come back to a world that's moved on.

**Stasis Field**
Freeze a target (or zone) in time for 1-2 turns. They take no damage, can't act, and block movement through their tile. The balance lever: stasis can protect enemies as easily as hurt them. Locking down a dangerous melee unit in the open is strong — but so is accidentally shielding a wounded enemy from a killshot.

**Acceleration**
Grants the caster bonus AP immediately — but queues an AP drain for next round. You're borrowing from the future: net positive this turn, net negative next turn. Generates flux.

**Entropic Rounds**
Unconditional hit that deals the attacker's raw attack stat as damage, ignoring defense and cover entirely. Reliable baseline damage at the cost of generating flux.

**Echo Strike**
Copies and immediately executes the last ability the target used, at zero AP cost to the caster. If the target has no recorded ability (or their last was EchoStrike), deals basic melee damage instead. Enormously disruptive when echoing a powerful enemy ability back at them.

**Temporal Recall** *(new)*
Snaps a target back to their position before their last move. Fails if the target hasn't moved. Clears their `LastPosition` so they can only be recalled once per move. Works on allies or enemies (`RangedAny` targeting).

---

## Balancing the Time Fantasy

This is the hard part. Time manipulation is fun until it feels broken or confusing. Here are the key design principles:

**Cost in "Temporal Flux"**
Chronurgy doesn't use ammo or mana — it generates **Flux**. Each Researcher ability has a `flux_generation` value; using it adds that amount to the zone's `TemporalFlux` resource (0–100). Above 75 flux, all hit rolls incur a scaling penalty (up to −20% at flux 100). At flux 76–100, the current actor risks a **glitch teleport** (random repositioning, frequency and distance increase with flux). At flux 100, a **Temporal Collapse** fires: one random combatant takes 15 damage, all combatants lose 2 AP, and flux resets to 0. The battlefield itself becomes unstable if you overuse it.

**Locality**
Time effects are  *zonal* , not global. A stasis field doesn't stop the whole battle, just a tile radius. This keeps the system legible — players can see clearly what's affected.

**No True Undoing**
Rewind is the dangerous one. The key rule: **you can rewind a unit's state, but not the world's knowledge.** Enemies that saw you before the rewind still have line-of-sight memory. An ally that died doesn't un-die. This prevents rewind from being a free undo button while keeping it tactically interesting.

**Chronurgy is Loud**
Temporal weapons have a visual and audio signature. Enemies react to it — some have Flux-sensitive detectors, others become aggressive or erratic near high-Flux zones. Using time powers is a strategic commitment that changes enemy behavior, not just a quiet edge.

**Diminishing Returns on Targets**
A unit hit by temporal effects becomes *temporally anchored* for a few turns — resistant to further manipulation. You can't chain-rewind or permanently stasis a key enemy. You get one window, then they stabilize.

---

## The Interesting Design Space

The best chronurgy moments would be emergent combinations:

> Accelerate an ally → they sprint into flanking position and fire → immediately Rewind the enemy back into the open → delayed damage from an Echo Strike hits them there

The counter-play is equally interesting — enemies might carry **Temporal Dampeners** that reduce Flux generation, or **Anchors** that make certain units immune to displacement, forcing players to neutralize those first.
