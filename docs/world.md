# World Layout

The Meridian is a science station built into the surface of a moon orbiting a dying star. Zones span from deep interior to open moon surface. All zones connect via doors on cardinal sides.

Each zone is **4–5 rooms** across **2–3 screens** (~100–200 tiles). The Station Exterior is the exception — it is a large open outdoor area, roughly 3 screens wide.

---

## Zone Map

```
                         [Relay Array]
                               |N
          [Excavation Site]--W[Station Exterior]
                                    |S
             [Command Deck]--W[Docking Bay]--E[Military Annex]
                   |S               |S               |S
             [Research Wing]--E[Systems Core]--E[Medical Bay]
```

---

## Interior Zones

### Research Wing
**Connections:** N → Command Deck, E → Systems Core
**Rooms:** Lab A, Lab B, Room 7-C (starting room), Anomaly Observation Chamber
**Size:** 4 rooms, ~120 tiles

The player starts here every loop. Room 7-C is the opening scene location. The Anomaly Observation Chamber is locked in loop 1, progressively more accessible as the player gains clearance or the station degrades.

**Encounters:**
- Maintenance Drones (Neutral loops 1–2; Aggressive loops 3+)
- Scavengers (loops 2+ — they've breached the outer hull)

**NPCs:**
- Salvage Operative (Friendly loops 1–2; trades supplies and station rumors)

---

### Command Deck
**Connections:** S → Research Wing, E → Docking Bay
**Rooms:** Bridge, Orin's Office, Briefing Room, Communications Array
**Size:** 4 rooms, ~130 tiles

Orin's domain. The mission briefing terminal is here. Her personal logs are locked behind terminal permissions. The locket detail is discoverable in Orin's Office starting loop 2.

**Encounters:**
- Station Guards (Friendly loops 1–2; Neutral loop 3; Aggressive loops 4–5)
- Security Units (loops 2+; patrol the Bridge)

**NPCs:**
- Dr. Sable Orin (present if not the chosen companion; gives the mission briefing)

---

### Military Annex
**Connections:** W → Docking Bay, S → Medical Bay
**Rooms:** Armory, Doss's Quarters, Training Area, Restricted Storage, Guard Post
**Size:** 5 rooms, ~160 tiles

Doss's domain. His war maps are discoverable in his quarters starting loop 2. The Restricted Storage requires either Doss's clearance or a forced entry (which turns nearby personnel Aggressive). The Armory has the best kinetic gear in the station.

**Encounters:**
- Gun-for-Hire (Neutral; can be bribed or turned)
- Shock Troopers (loops 3+; loyal to Doss; always Aggressive)

**NPCs:**
- Recruiter Doss (present if not the chosen companion; antagonistic early, deteriorating late)

---

### Systems Core
**Connections:** W → Research Wing, E → Medical Bay, N → Docking Bay
**Rooms:** Server Room, Power Core, Kaleo Terminal Hub, Maintenance Corridor
**Size:** 4 rooms, ~140 tiles

Kaleo's domain. The Terminal Hub is where Kaleo's status appears in loop 2 and where the recruitment message appears in loop 3. Breaking the permission wall on the Server Room in loop 4–5 reveals Kaleo's full iteration log.

**Encounters:**
- Maintenance Drones (Neutral loops 1–2; Aggressive loops 3+)
- Security Units (loops 2+; enhanced sensors each loop)

**NPCs:**
- Unit Kaleo (via terminals; physically present here if not recruited)

---

### Medical Bay
**Connections:** N → Military Annex, W → Systems Core
**Rooms:** Treatment Ward, Supply Closet, Surgery, Triage
**Size:** 4 rooms, ~120 tiles

A side path — not required to complete any loop. Best source of consumables. In loops 4–5, Void Spitters breach from below through the Surgery floor, drawn by the flux readings.

**Encounters:**
- Void Spitters (loops 4–5; breach through floor)
- Abyssal Brute (loop 5 only; rare)

**NPCs:**
- Salvage Operative (Friendly loops 1–2; Neutral loop 3)
- Station Guard (Friendly loops 1–2; gone or Aggressive loops 3+)

---

### Docking Bay
**Connections:** N → Station Exterior, E → Military Annex, W → Command Deck, S → Systems Core
**Rooms:** Main Bay, Airlock, Equipment Staging
**Size:** 3 rooms, ~110 tiles

The hub between interior and exterior. The Airlock requires a suit in loop 1; in loops 3+ the hull integrity is bad enough that the seal is broken and you pass through freely. Drifters and Void Raiders first appear here — they came in through the docking ports.

**Encounters:**
- Scavengers (loops 1+; first Drifter encounter)
- Void Raiders (loops 2+; ranged; use the bay's vertical cover)
- Drifter Boss (loops 3+)

**NPCs:**
- Gun-for-Hire (Neutral; offers safe passage intel for payment)

---

## Exterior Zones

### Station Exterior *(large outdoor zone)*
**Connections:** S → Docking Bay, E → Excavation Site, N → Relay Array
**Rooms:** Open moon surface — no discrete rooms, 3 screens wide
**Size:** ~200 tiles

The largest zone. Low gravity affects movement. The dying star is visible on the horizon. In loops 1–2, it's quiet — just wind and distant geological noise. By loop 4, sections of the surface are temporally displaced: patches of ground flicker between their current state and collapsed rubble. Moon Crawlers hunt in packs across the open ground.

**Encounters:**
- Moon Crawlers (loops 1+; fast packs; dangerous in open ground)
- Void Raiders (loops 2+; sniping from elevated rock formations)
- Abyssal Brutes (loops 3+; slow but enormous; cross the open from the excavation direction)

**NPCs:**
- Salvage Operative (Neutral loop 3; last one alive outside; trading for passage off-moon)

---

### Relay Array
**Connections:** S → Station Exterior
**Rooms:** Main Dish Platform, Control Shack, Antenna Field, Observation Post
**Size:** 4 rooms/areas, ~130 tiles

The communications array. Dead in loop 1 — no signal out. In loops 2–3 the player can partially restore it and receive fragmented transmissions (flavor lore: the war, other stations). In loop 4 the transmissions become temporally displaced — the player receives messages from before the loop started and from iterations that didn't happen.

**Encounters:**
- Void Raiders (all loops; this is their main camp)
- Drifter Boss (loops 2+; commands the raider camp)

**NPCs:** None

---

### Excavation Site
**Connections:** W → Station Exterior
**Rooms:** Surface Dig, Entry Tunnel, Lower Chamber, Anomaly Vent, Collapsed Section
**Size:** 5 rooms, ~150 tiles

Where the anomaly was originally found. The deepest temporal instability in the game — even in loop 1, something feels wrong here. The Anomaly Vent pulses with flux and is the only place Abyssal Fauna appear in loop 1. The Collapsed Section is impassable in loops 1–2; it opens in loop 3 as bleed damage clears the path, revealing logs from the original excavation team.

**Encounters:**
- Moon Crawlers (all loops)
- Void Spitters (loops 2+)
- Abyssal Brute (loops 2+; guards the Lower Chamber)

**NPCs:** None (excavation team logs are readable items, not characters)

---

## Zone Summary

| Zone | Type | Rooms | Tiles | Key NPC | Encounters |
|---|---|---|---|---|---|
| Research Wing | Interior | 4 | ~120 | Salvage Operative | Drones, Scavengers |
| Command Deck | Interior | 4 | ~130 | Dr. Orin | Guards, Security Units |
| Military Annex | Interior | 5 | ~160 | Doss | Gun-for-Hire, Shock Troopers |
| Systems Core | Interior | 4 | ~140 | Kaleo | Drones, Security Units |
| Medical Bay | Interior | 4 | ~120 | Salvage Operative | Void Spitters (late) |
| Docking Bay | Interior | 3 | ~110 | Gun-for-Hire | Scavengers, Raiders, Boss |
| Station Exterior | Exterior | open | ~200 | Salvage Operative | Crawlers, Raiders, Brutes |
| Relay Array | Exterior | 4 | ~130 | — | Raiders, Drifter Boss |
| Excavation Site | Exterior | 5 | ~150 | — | Crawlers, Spitters, Brute |
