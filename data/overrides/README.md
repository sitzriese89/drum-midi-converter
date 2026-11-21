# Per-pair override tables (ADR-001).
#
# One TOML file per directed pair: `<from>-to-<to>.toml`, with rules
# `[[rules]] from = "<tag>" to = "<tag>"`. See crates/drum-map/src/overrides.rs
# for the full schema. No built-in pair needs an override yet.
