<script lang="ts">
  // The conversion report, table-first (SPEC §4.3, Q10): one row per used note — instrument
  // name, hit count, mapped target, LOSSY badge where a fallback fired, and an override
  // dropdown for unmapped AND lossy notes (ADR-004, SPEC §4.3).
  import type { NoteRow } from '../drum-map';
  import type { OverrideChoice } from '../overrides';

  interface Props {
    rows: NoteRow[];
    overrides: Map<number, OverrideChoice>;
    targetNotes: { note: number; name: string }[];
    onOverride?: (note: number, choice: OverrideChoice) => void;
  }

  let { rows, overrides, targetNotes, onOverride }: Props = $props();

  function choiceKey(note: number): string {
    const choice = overrides.get(note);
    if (!choice || choice.kind === 'keep') return 'keep';
    return choice.kind === 'drop' ? 'drop' : `reassign:${choice.note}`;
  }

  function setChoice(note: number, key: string) {
    if (!onOverride) return;
    if (key.startsWith('reassign:')) {
      onOverride(note, { kind: 'reassign', note: Number(key.slice('reassign:'.length)) });
    } else if (key === 'drop') {
      onOverride(note, { kind: 'drop' });
    } else {
      onOverride(note, { kind: 'keep' });
    }
  }

  /** Target label for a report row: the mapped target, or the effective override. */
  function targetLabel(row: NoteRow): string {
    const choice = overrides.get(row.source_note);
    if (choice?.kind === 'drop') return 'dropped';
    if (choice?.kind === 'reassign') {
      const target = targetNotes.find((t) => t.note === choice.note);
      return `→ ${choice.note}${target ? ` ${target.name}` : ''}`;
    }
    // An explicit keep (or an unmapped row's default) keeps the source note as-is.
    if (row.unmapped || choice?.kind === 'keep') return 'kept as-is';
    return row.target_note !== null && row.target_note !== undefined
      ? `${row.target_note} ${row.target_name ?? ''}`.trim()
      : '—';
  }
</script>

<table class="report" data-testid="report-table">
  <thead>
    <tr>
      <th scope="col">Source</th>
      <th scope="col">Instrument</th>
      <th scope="col">Hits</th>
      <th scope="col">Target</th>
      <th scope="col">Status</th>
      <th scope="col">Override</th>
    </tr>
  </thead>
  <tbody>
    {#each rows as row (row.source_note)}
      <tr data-testid="report-row" data-note={row.source_note}>
        <td>{row.source_note}</td>
        <td>{row.source_name}</td>
        <td>{row.hit_count}</td>
        <td>{targetLabel(row)}</td>
        <td>
          {#if row.lossy}
            <span class="badge lossy" data-testid="lossy-badge">LOSSY</span>
          {:else if row.unmapped}
            <span class="badge unmapped" data-testid="unmapped-badge">UNMAPPED</span>
          {:else}
            <span class="badge exact" data-testid="exact-badge">exact</span>
          {/if}
        </td>
        <td>
          {#if row.unmapped || row.lossy}
            <select
              aria-label={`Override for note ${row.source_note}`}
              value={choiceKey(row.source_note)}
              onchange={(e) => setChoice(row.source_note, e.currentTarget.value)}
            >
              <option value="keep">keep as-is</option>
              <option value="drop">drop</option>
              {#each targetNotes as t (t.note)}
                <option value={`reassign:${t.note}`}>{t.note} {t.name}</option>
              {/each}
            </select>
          {:else}
            <span class="no-override">—</span>
          {/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>
