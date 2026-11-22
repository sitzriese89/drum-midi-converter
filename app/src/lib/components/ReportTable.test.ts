import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import ReportTable from './ReportTable.svelte';
import type { NoteRow } from '../drum-map';
import type { OverrideChoice } from '../overrides';

function row(overrides: Partial<NoteRow>): NoteRow {
  return {
    source_note: 36,
    source_name: 'Kick',
    hit_count: 1,
    target_note: 24,
    target_name: 'Kick',
    lossy: false,
    unmapped: false,
    ...overrides,
  };
}

describe('ReportTable', () => {
  it('renders one row per used note with names, hits and targets', () => {
    render(ReportTable, {
      rows: [row({}), row({ source_note: 38, source_name: 'Snare', target_note: 26 })],
      overrides: new Map(),
      targetNotes: [],
    });
    expect(screen.getAllByTestId('report-row')).toHaveLength(2);
    expect(screen.getByText('Kick')).toBeInTheDocument();
    expect(screen.getByText('24 Kick')).toBeInTheDocument();
  });

  it('marks fallback conversions with a LOSSY badge', () => {
    render(ReportTable, {
      rows: [row({ lossy: true }), row({ source_note: 38, source_name: 'Snare' })],
      overrides: new Map(),
      targetNotes: [],
    });
    expect(screen.getByTestId('lossy-badge')).toBeInTheDocument();
    expect(screen.getAllByTestId('exact-badge')).toHaveLength(1);
  });

  it('offers keep/drop/reassign overrides for unmapped notes (ADR-004)', async () => {
    const onOverride = vi.fn<(note: number, choice: OverrideChoice) => void>();
    render(ReportTable, {
      rows: [row({ unmapped: true, target_note: null, target_name: null })],
      overrides: new Map(),
      targetNotes: [
        { note: 24, name: 'Kick' },
        { note: 26, name: 'Snare' },
      ],
      onOverride,
    });
    const select = screen.getByRole('combobox', { name: /override for note 36/i });
    expect(screen.getByTestId('unmapped-badge')).toBeInTheDocument();
    await fireEvent.change(select, { target: { value: 'drop' } });
    expect(onOverride).toHaveBeenCalledWith(36, { kind: 'drop' });
    await fireEvent.change(select, { target: { value: 'reassign:26' } });
    expect(onOverride).toHaveBeenCalledWith(36, { kind: 'reassign', note: 26 });
    await fireEvent.change(select, { target: { value: 'keep' } });
    expect(onOverride).toHaveBeenCalledWith(36, { kind: 'keep' });
  });

  it('shows the effective override for an unmapped note', () => {
    render(ReportTable, {
      rows: [row({ unmapped: true, target_note: null, target_name: null })],
      overrides: new Map<number, OverrideChoice>([[36, { kind: 'reassign', note: 26 }]]),
      targetNotes: [],
    });
    expect(screen.getByText(/→ 26/)).toBeInTheDocument();
  });

  it('gives mapped notes no override dropdown', () => {
    render(ReportTable, {
      rows: [row({})],
      overrides: new Map(),
      targetNotes: [],
    });
    expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
  });
});
