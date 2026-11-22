<script lang="ts">
  // The conversion flow (SPEC §4): file in, source/target dropdowns with detection hint and
  // mismatch warning, convert, table-first report with overrides, save + drag-out (Q13).
  import { version } from './lib/drum-map';
  import { Converter, toMidiFile } from './lib/converter.svelte';
  import ReportTable from './lib/components/ReportTable.svelte';
  import { SvelteMap } from 'svelte/reactivity';

  const converter = new Converter();
  converter.loadCatalog();

  let outputName = $state('converted.mid');
  let draggingOver = $state(false);
  let dragUrl = $state('');

  const targetNotes = $derived.by(() => {
    const target = converter.maps.find((m) => m.id === converter.toId);
    if (!target) return [];
    // The override dropdowns offer the target vocabulary seen in the report: the target
    // notes the conversion actually used, with their names. Reassign options come from
    // there; enough for v1 (full target-map listing arrives with the custom maps, M5).
    return noteVocabulary();
  });

  const hintMapId = $derived(converter.hint?.map_id ?? '');
  const coveragePercent = $derived(
    converter.coverage === null ? 100 : Math.round(converter.coverage * 100),
  );

  function noteVocabulary(): { note: number; name: string }[] {
    const entries = new SvelteMap<number, string>();
    for (const row of converter.report?.rows ?? []) {
      if (row.target_note !== null && row.target_note !== undefined) {
        entries.set(row.target_note, row.target_name ?? '');
      }
    }
    return [...entries.entries()]
      .map(([note, name]) => ({ note, name }))
      .sort((a, b) => a.note - b.note);
  }

  async function onDrop(event: DragEvent) {
    event.preventDefault();
    draggingOver = false;
    const file = event.dataTransfer?.files[0];
    if (file) await converter.loadFile(await toMidiFile(file));
  }

  function onDragOver(event: DragEvent) {
    event.preventDefault();
    draggingOver = true;
  }

  async function onDragOut(event: DragEvent) {
    if (!converter.converted) return;
    dragUrl = (await converter.dragOut(outputName)) ?? '';
    event.dataTransfer?.setData('DownloadURL', `audio/midi:${outputName}:${dragUrl}`);
    event.dataTransfer?.setData('text/uri-list', dragUrl);
  }

  function suggestName(): string {
    const base = converter.file?.name.replace(/\.midi?$/i, '') ?? 'converted';
    const to = converter.maps.find((m) => m.id === converter.toId)?.name ?? '';
    return `${base} (${to}).mid`.replace(/[/\\:*?"<>|]/g, '-');
  }

  async function onSave() {
    outputName = suggestName();
    await converter.save(outputName);
  }
</script>

<main>
  <h1>Drum MIDI Converter</h1>

  <section
    class="dropzone"
    class:active={draggingOver}
    aria-label="MIDI file drop zone"
    ondragover={onDragOver}
    ondragleave={() => (draggingOver = false)}
    ondrop={onDrop}
  >
    <p>
      <button onclick={() => converter.pickFile()}>Choose MIDI file…</button>
      <input
        type="file"
        accept=".mid,.midi,audio/midi"
        aria-label="Choose MIDI file"
        onchange={async (e) => {
          const file = e.currentTarget.files?.[0];
          if (file) await converter.loadFile(await toMidiFile(file));
        }}
      />
      or drag a .mid file here
    </p>
    {#if converter.file}
      <p>Loaded: <strong>{converter.file.name}</strong></p>
    {/if}
  </section>

  {#if converter.hint}
    <p class="hint-chip" data-testid="hint-chip">
      Looks like {hintMapId}
      <button onclick={() => converter.setFromId(hintMapId)}>Use as source</button>
    </p>
  {/if}

  <section class="picks">
    <label>
      Source map
      <select value={converter.fromId} onchange={(e) => converter.setFromId(e.currentTarget.value)}>
        {#each converter.maps as map (map.id)}
          <option value={map.id}>{map.name}</option>
        {/each}
      </select>
    </label>
    <label>
      Target map
      <select bind:value={converter.toId}>
        {#each converter.maps as map (map.id)}
          <option value={map.id}>{map.name}</option>
        {/each}
      </select>
    </label>
    <button onclick={() => converter.convert()}>Convert</button>
  </section>

  {#if converter.mismatch}
    <p class="warning" role="alert" data-testid="mismatch-warning">
      Not every used note is in the chosen source map ({coveragePercent}% covered). Unmapped notes
      are kept unless you override them.
    </p>
  {/if}

  {#if converter.error}
    <p class="error" role="alert">{converter.error}</p>
  {/if}

  {#if converter.report}
    <section class="report">
      <h2>Report</h2>
      <ReportTable
        rows={converter.report.rows}
        overrides={converter.overrides}
        {targetNotes}
        onOverride={(note, choice) => {
          converter.setOverride(note, choice);
          converter.convert();
        }}
      />
      <div class="output">
        <button onclick={onSave}>Save converted file…</button>
        {#if converter.converted}
          <a href={dragUrl} draggable="true" data-testid="drag-out" ondragstart={onDragOut}>
            Drag me into your DAW
          </a>
        {/if}
      </div>
    </section>
  {/if}

  <footer>drum-map core v{version()}</footer>
</main>
