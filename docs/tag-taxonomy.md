# Tag taxonomy (ADR-001 vocabulary)

Canonical semantic articulation tags for all drum maps. Every map file uses
these; new concepts extend the list, they don't invent parallel names.
Segments: lowercase ascii alphanumerics plus `-`, joined by `.`
(most general → most specific).

## Principles

- **One tag per note, unique within a map** (enforced at parse time).
  Duplicate triggers of the same articulation (common in Toontrack maps) get
  a numeric suffix: `snare.center.2`, `snare.center.3`, … The un-suffixed
  tag marks the primary note (the GM-style one when there is a choice).
- **Parent-walk friendliness.** Structure tags so dropping the last segment
  yields a musically sensible less-specific articulation:
  `hihat.open.3.tip` → `hihat.open.3` → `hihat.open` → `hihat`.
  `crash.2.mute` → `crash.2` → `crash`.
- GM (gm.toml) intentionally uses bare/pitched tags (`hihat.open`,
  `tom.high`, `crash.1`) — it is the floor every ladder collapses toward.

## Families and tags

### Kick
`kick.1` (main), `kick.2` (alt), `kick.3` …, `kick.left` (left beater),
`kick.acoustic` (GM 35).

### Snare
`snare.center` (main hit), `snare.2` (second snare drum),
`snare.side` (side snare), `snare.rimshot`, `snare.rim`, `snare.sidestick`,
`snare.flam`, `snare.ruff`, `snare.off` (snares off),
`snare.electric` (GM 40; GM 38 "Acoustic Snare" uses the canonical
`snare.center`).
Duplicate triggers: `snare.center.2`, `snare.sidestick.2`, …

### Toms
`tom.rack.1..n` (1 = smallest/highest), `tom.floor.1..n` (1 = smallest floor).
Articulations: `tom.rack.2.rim`, `tom.floor.1.rimshot`.
Duplicates: `tom.floor.1.2`.
GM keeps pitched names: `tom.high`, `tom.mid-high`, `tom.mid-low`,
`tom.low`, `tom.floor.high`, `tom.floor.low`.

### Hi-hat
Openness ladder (tight < closed < loose < open 1..n):
`hihat.tight.tip`, `hihat.tight.edge`,
`hihat.closed.tip`, `hihat.closed.edge`, `hihat.closed.bell`,
`hihat.loose.tip`, `hihat.loose.edge`, `hihat.loose.bell`,
`hihat.open.N.tip` / `.edge` / `.bell` (N = 1..5),
`hihat.open.N` (maps without tip/edge split, e.g. GGD).
Pedal: `hihat.pedal`, `hihat.pedal.splash`.
CC-driven hats: `hihat.cc` (single note, GGD), `hihat.cc.tip`,
`hihat.cc.edge`, `hihat.cc.bell`.
Aux/second hat: `hihat.aux.1.<ladder tag>` (e.g. `hihat.aux.1.closed.tip`).
GM floor: `hihat.closed`, `hihat.open`, `hihat.pedal`.

### Ride
`ride.1` (main bow), `ride.2`, `ride.bow`, `ride.bow.tip`, `ride.edge`,
`ride.bell`, `ride.bell.tip`, `ride.mute`.

### Crashes
`crash.1..n`, articulations `crash.N.mute`, `crash.N.bow`,
`crash.N.bow.tip`, `crash.N.bell`, `crash.N.bell.tip`.
GM variants: `crash.chinese`, `crash.splash` (GM map only — real maps use
the `china` / `splash` families below).

### China / Splash / Stack / Bell (dedicated cymbals)
`china.1..n`, `china.N.mute`, `china.N.bow`, `china.N.bow.tip`,
`china.N.bell`, `china.N.bell.tip`.
`splash.1..n`, `splash.N.mute`.
`stack.1..n`, `stack.N.mute`.
`bell.1..n` (dedicated bell cymbal, not a ride bell), `bell.N.mute`.

### Percussion / extras
`clap.1`, `shaker.1`, `percussion.tambourine`, `percussion.cowbell`,
`percussion.1` (generic, SD3), plus the GM-only names already in gm.toml.

## Fallback rules (per source map, `[fallback]` section)

Parent-walk handles mutes and ladder collapses automatically. Add explicit
`[[fallback.rules]]` in the source map for cross-family collapses and
duplicate-trigger fold-ins, e.g.:
`kick.left → kick.1`, `snare.flam → snare.center`,
`snare.center.2 → snare.center`, `china.1 → crash.1`,
`splash.1 → crash.1`, `stack.1 → crash.1`, `bell.1 → ride.bell`,
`hihat.aux.1.closed → hihat.closed`, `hihat.cc → hihat.closed`,
`hihat.open.5.tip → hihat.open.3.tip` (when the map's ladder exceeds the
commonly available depth).
