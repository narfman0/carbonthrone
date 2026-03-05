## NPC & Enemy Design

Enemies are grouped into five factions. Within each faction, multiple variants cover a range of combat roles, giving each loop several encounters that feel distinct while sharing thematic identity.

Aggression states:
- **Aggressive** — attacks the party on sight
- **Neutral** — ignores the party unless provoked or bribed
- **Friendly** — will not initiate combat; may trade or give information
- **Lethargic** — passive and slow; a degraded form of Aggressive that occurs in late loops when temporal instability affects simpler organisms

---

## The Constancy

A fervent order that believes temporal manipulation is a fundamental violation — of nature, of causality, of something they call "the given shape of things." They are not primitive or irrational. Many are scientists, engineers, and soldiers who understand exactly what temporal technology does and have concluded it must be destroyed. Their doctrine holds that no cause justifies rewriting time, because the act of rewriting it is itself the catastrophe.

They are the ones cutting through the bulkhead in Room 7-C. They initiated the attack that opens the game.

In early loops they read as fanatics. By loop 3, the player starts finding their literature and understanding their reasoning. By loop 4, a captured Constancy member can be spoken to. By loop 5, the player may realize that The Constancy and Sable Orin want the same thing — and that The Constancy has been trying to do what Sable is doing, from outside, for longer than the current loop.

**Mechanical identity:** The Constancy carry **Temporal Dampeners** — devices that reduce Flux generation from allied abilities. Fighting them suppresses the player's temporal toolkit. They are specifically designed as a counter to the Researcher's class strengths.

| Variant | Role | Default Aggression | Notes |
|---|---|---|---|
| **Zealot** | Fast melee; rushes in; glass cannon | Aggressive | The true believers; first enemies encountered in the game |
| **Preacher** | Support; buffs nearby Constancy; aura suppresses Flux generation | Aggressive | Priority target; removing them restores temporal ability effectiveness |
| **Purifier** | Ranged; anti-temporal rounds that deal bonus damage to high-Flux units | Aggressive | Punishes heavy temporal ability use; incentivizes Flux discipline |
| **Archon** | Boss; heavily armored leader; carries a Dampener powerful enough to suppress zone-wide Flux | Aggressive | One per major encounter; has dialogue in loops 3+; recognizes the player in loop 5 |

Loop behavior: The Constancy are the only faction whose aggression does not change — they are always Aggressive. What changes is the player's understanding of them. In loop 3, a wounded Zealot can be left alive and will speak before dying. In loop 4, the Archon will pause before the fight to address the player directly — briefly, coldly — acknowledging that they've met before. In loop 5, if the player has found Sable's full truth, a new dialog option appears with the Archon: *"Then you know we're right."*

---

## Drifters

Scavengers and opportunists who arrived on the moon chasing salvage. Aggressive by default. No loyalty — they attack anyone who looks like a threat or a target.

| Variant | Role | Default Aggression | Notes |
|---|---|---|---|
| **Scavenger** | Fast light melee; packs of 2–3 | Aggressive | Glass cannon; first encounter in most loops |
| **Void Raider** | Ranged gunslinger | Aggressive | Appears mid-loop; hangs back and suppresses |
| **Drifter Boss** | Heavy bruiser, pack leader | Aggressive | One per encounter group; buffs nearby Scavengers |

Loop behavior: Drifters are consistent across all loops — they never become friendly. In Loop 4–5, some Void Raiders defect and can be found trading intel for safe passage (Neutral).

---

## Automata

Repurposed station robotics. Originally built for maintenance and security; the temporal instability has corrupted their behavior protocols.

| Variant | Role | Default Aggression | Notes |
|---|---|---|---|
| **Maintenance Drone** | Confused melee; erratic pathing | Neutral | Won't attack unless the player enters its defined "work zone" |
| **Security Unit** | Ranged patrol; fires on unauthorized presence | Aggressive | Loops 2+ only; Loop 1 Security Units are Neutral (not yet corrupted) |
| **Combat Frame** | Military-grade mech; boss tier | Aggressive | Rare; found near restricted zones; drops high-tier loot |

Loop behavior: Automata become progressively more aggressive as the station degrades. A Maintenance Drone that was Neutral in Loop 1 may be Aggressive by Loop 4. Security Units gain enhanced sensors each loop.

---

## Abyssal Fauna

Native creatures from the moon's subsurface. The temporal anomaly disturbs their behavior. Normally reclusive, they surface when reality destabilizes.

| Variant | Role | Default Aggression | Notes |
|---|---|---|---|
| **Moon Crawler** | Fast pack melee; glass cannon | Aggressive | Fastest enemy in the game; dies easily; dangerous in numbers |
| **Void Spitter** | Ranged bio attack; magic-adjacent damage | Aggressive | Hangs back; targets low-defense units |
| **Abyssal Brute** | Slow tanky apex predator | Aggressive | Rare early; common in Loops 4–5 |

Loop behavior: In Loops 4–5, as temporal flux peaks, Abyssal Fauna become **Lethargic** — their aggression collapses, they move slowly, and they will not initiate combat. Unit Kaleo notes in its logs that the creatures seem to be "retreating from time itself." This is the only faction that can turn passive through loop progression alone.

---

## Station Personnel

Crew and contractors aboard The Meridian. Friendly in early loops. Temporal sickness, paranoia, and Doss's influence corrupt them over time.

| Variant | Role | Default Aggression | Notes |
|---|---|---|---|
| **Salvage Operative** | Lightly armed mercenary; trades intel and supplies | Friendly | Becomes Neutral in Loop 3, Aggressive in Loop 5 when desperate |
| **Gun-for-Hire** | Armed contractor; neutral until bribed or threatened | Neutral | Can be paid to assist or to stand down; turns Aggressive if Doss reaches them |
| **Station Guard** | Station security; knows the layout | Friendly | Loops 1–2 friendly; Loop 3+ Neutral; Loop 4–5 Aggressive due to temporal delirium |
| **Shock Trooper** | Military enforcer loyal to the weapons program | Aggressive | Always hostile; deployed by Doss in Loops 3–5 |

Loop behavior: Personnel aggression follows the narrative arc. Recruiter Doss actively flips Neutral personnel to Aggressive in later loops, treating them as conscripts in his effort to preserve the timeline. A player who finds Doss early can interrupt this.

---

## Design Notes

**Encounter composition per loop (suggested):**
- Loop 1: **Constancy Zealots + Purifiers (opening breach)**, Scavengers, Maintenance Drones, Station Guards (Friendly), Moon Crawlers
- Loop 2: Adds Void Raiders, Security Units (corrupted), Void Spitters, Constancy Preachers
- Loop 3: Adds Drifter Bosses, Gun-for-Hire (Neutral), Station Guards going hostile; wounded Zealot has dialog
- Loop 4: Adds Shock Troopers, Abyssal Brutes; Moon Crawlers turn Lethargic; Archon addresses the player
- Loop 5: Combat Frames active; Abyssal Fauna Lethargic; Station Personnel fully hostile; Archon has full dialog branch

**Aggression as a resource:** Friendly and Neutral NPCs are not just avoided fights — they're intel sources, traders, and narrative hooks. Turning them hostile early (by attacking first) closes off those options. This creates a reason to manage aggression carefully rather than fighting everything.
