// End-to-end glue test: the real WASM core (loaded in test-setup.ts) converts a minimal
// SMF and ranks the catalog (SPEC §4.1/§4.2).
import { describe, expect, it } from 'vitest';
import { convertMidi, detectHints, listMaps, mapNotes, sourceCoverage, version } from './drum-map';

/** Format-0 SMF: one kick hit (note 36) plus an unmapped cowbell (56) on channel 10. */
function drumMidi(): Uint8Array {
  return new Uint8Array([
    0x4d,
    0x54,
    0x68,
    0x64, // "MThd"
    0,
    0,
    0,
    6, // header length
    0,
    0, // format 0
    0,
    1, // one track
    0x01,
    0xf4, // division 480
    0x4d,
    0x54,
    0x72,
    0x6b, // "MTrk"
    0,
    0,
    0,
    15, // track length
    0,
    0x99,
    36,
    100, // note-on 36, ch 10
    0x60,
    0x89,
    36,
    64, // note-off 36
    0,
    0x99,
    56,
    80, // note-on 56 (cowbell, unmapped)
    0x60,
    0x89,
    56,
    64, // note-off 56
    0x60,
    0xff,
    0x2f,
    0x00, // end of track
  ]);
}

describe('drum-map WASM', () => {
  it('exposes the core version and the built-in catalog', () => {
    expect(version()).toMatch(/^\d+\.\d+\.\d+$/);
    const maps = listMaps();
    expect(maps.length).toBeGreaterThanOrEqual(7);
    expect(maps.map((m: { id: string }) => m.id)).toContain('ggd-invasion');
    expect(maps.every((m: { id: string; name: string }) => m.id && m.name)).toBe(true);
  });

  it('ranks detection candidates and reports source coverage', () => {
    const hints = detectHints(drumMidi());
    expect(hints.length).toBeGreaterThanOrEqual(7);
    expect(hints[0].confidence).toBeGreaterThan(0);
    expect(sourceCoverage(drumMidi(), 'gm')).toBeCloseTo(1.0);
    // Some catalog map lacks at least one of the used notes (mismatch case).
    const maps = listMaps();
    expect(maps.some((m: { id: string }) => (sourceCoverage(drumMidi(), m.id) ?? 1) < 1)).toBe(
      true,
    );
  });

  it('converts GM → GGD Invasion and reports the lossy/unmapped rows', () => {
    const result = convertMidi(drumMidi(), 'gm', 'ggd-invasion', null);
    const kick = result.report.rows.find((r: { source_note: number }) => r.source_note === 36);
    expect(kick.target_note).toBe(24);
    const cowbell = result.report.rows.find((r: { source_note: number }) => r.source_note === 56);
    expect(cowbell.unmapped).toBe(true);
    // The output is valid SMF: the kick note-on became Invasion note 24.
    const bytes = result.midi;
    expect(bytes[0]).toBe(0x4d); // "M"
    expect(Array.from(bytes)).toContain(24);
  });

  it('applies per-note overrides (drop) and re-runs cleanly', () => {
    const result = convertMidi(drumMidi(), 'gm', 'ggd-invasion', { 56: { kind: 'drop' } });
    const cowbell = result.report.rows.find((r: { source_note: number }) => r.source_note === 56);
    expect(cowbell.unmapped).toBe(true);
    expect(cowbell.target_note).toBeNull();
  });

  it('offers the FULL target-map note vocabulary (SPEC §4.2)', () => {
    const notes = mapNotes('gm');
    // Not just the notes a particular conversion used: the whole map.
    expect(notes.length).toBe(47);
    expect(notes.map((n: { note: number }) => n.note)).toEqual(
      [...notes].map((n) => n.note).sort((a, b) => a - b),
    );
    expect(notes.find((n: { note: number }) => n.note === 36)?.name).toBe('Bass Drum 1');
  });

  it('lets mapped and lossy rows be overridden too (SPEC §4.3)', () => {
    // 36 maps exactly; an explicit reassign must still win.
    const result = convertMidi(drumMidi(), 'gm', 'ggd-invasion', {
      36: { kind: 'reassign', note: 60 },
    });
    const kick = result.report.rows.find((r: { source_note: number }) => r.source_note === 36);
    expect(kick.target_note).toBe(60);
    expect(kick.unmapped).toBe(false);
    expect(Array.from(result.midi)).toContain(60);
  });

  it('rejects a malformed per-note key', () => {
    expect(() => convertMidi(drumMidi(), 'gm', 'ggd-invasion', { abc: { kind: 'drop' } })).toThrow(
      /invalid note number/,
    );
  });
});
