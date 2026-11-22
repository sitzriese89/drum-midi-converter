// The conversion flow state (SPEC §4.1–§4.3): detection stays a hint (ADR-003),
// the target vocabulary is the FULL target map (SPEC §4.2). Runs against the real
// WASM core loaded in test-setup.ts.
import { describe, expect, it } from 'vitest';
import { Converter } from './converter.svelte';

/** Format-0 SMF: one kick hit (36) plus an unmapped cowbell (56) on channel 10. */
function drumMidi(): Uint8Array {
  return new Uint8Array([
    0x4d, 0x54, 0x68, 0x64, 0, 0, 0, 6, 0, 0, 0, 1, 0x01, 0xf4, 0x4d, 0x54, 0x72, 0x6b, 0, 0, 0, 15,
    0, 0x99, 36, 100, 0x60, 0x89, 36, 64, 0, 0x99, 56, 80, 0x60, 0x89, 56, 64, 0x60, 0xff, 0x2f,
    0x00,
  ]);
}

describe('Converter', () => {
  it('defaults the source to the first catalog map and loads the full target vocabulary', async () => {
    const converter = new Converter();
    await converter.loadCatalog();
    expect(converter.fromId).toBe(converter.maps[0].id);
    expect(converter.toId).toBe(converter.maps[1].id);
    // The full target-map note list, not just notes a conversion used (SPEC §4.2).
    expect(converter.targetNotes.length).toBeGreaterThan(10);
  });

  it('never pre-selects the source from detection (ADR-003)', async () => {
    const converter = new Converter();
    await converter.loadCatalog();
    const before = converter.fromId;
    await converter.loadFile({ name: 'test.mid', bytes: drumMidi() });
    // The hint is computed and shown, but the dropdown is untouched: the user
    // decides via the chip's Apply button.
    expect(converter.hint).not.toBeNull();
    expect(converter.fromId).toBe(before);
  });

  it('switching the target map refreshes the vocabulary', async () => {
    const converter = new Converter();
    await converter.loadCatalog();
    const before = converter.targetNotes.map((n) => n.note);
    const other = converter.maps.find((m) => m.id !== converter.toId)!;
    await converter.setToId(other.id);
    expect(converter.toId).toBe(other.id);
    expect(converter.targetNotes.map((n) => n.note)).not.toEqual(before);
  });

  it('override choices for lossy rows reach the conversion', async () => {
    const converter = new Converter();
    await converter.loadCatalog();
    await converter.loadFile({ name: 'test.mid', bytes: drumMidi() });
    // Force a lossy conversion source: GM kick (36) -> Invasion (24) is exact;
    // the cowbell (56) is unmapped. Reassign the unmapped note and convert.
    converter.setOverride(56, { kind: 'reassign', note: 60 });
    await converter.convert();
    const cowbell = converter.report!.rows.find((r) => r.source_note === 56)!;
    expect(cowbell.target_note).toBe(60);
  });
});
