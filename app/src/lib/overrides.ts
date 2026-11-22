// Per-note overrides for unmapped notes (ADR-004), as the UI keeps them and as the WASM
// conversion call expects them.

/** One user decision for an unmapped note. */
export type OverrideChoice =
  { kind: 'keep' } | { kind: 'drop' } | { kind: 'reassign'; note: number };

/** The default for every note without an override (ADR-004: keep, never silently drop). */
export const DEFAULT_CHOICE: OverrideChoice = { kind: 'keep' };

/** Builds the `per_note` object for `convertMidi` from the override map. */
export function overridesPayload(
  overrides: Map<number, OverrideChoice>,
): Record<number, OverrideChoice> {
  return Object.fromEntries(
    [...overrides.entries()].filter(([, choice]) => choice.kind !== 'keep'),
  );
}
