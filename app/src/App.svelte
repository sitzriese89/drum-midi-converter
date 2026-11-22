<script lang="ts">
  // The conversion flow (SPEC §4): file in, source/target dropdowns with detection hint and
  // mismatch warning, convert, table-first report with overrides, save + drag-out (Q13).
  // The map editor (SPEC §3.4, Q6) lives behind the "Map editor" tab.
  import { listMaps, version } from './lib/drum-map';
  import { Converter } from './lib/converter.svelte';
  import { readMidiFile } from './lib/files';
  import { UserMaps } from './lib/user-maps.svelte';
  import { MapEditor } from './lib/map-editor.svelte';
  import ReportTable from './lib/components/ReportTable.svelte';
  import MapEditorView from './lib/components/MapEditor.svelte';

  const userMaps = new UserMaps();
  const editor = new MapEditor(userMaps);
  const converter = new Converter();

  let view = $state<'convert' | 'maps'>('convert');

  // Load the user maps, then the catalog merged over them; every change to the
  // stored maps (save, delete) re-merges them into the source/target dropdowns.
  userMaps.load().then(async () => {
    editor.builtins = await listMaps();
    await converter.loadCatalog(userMaps.maps);
  });
  $effect(() => {
    // Re-run whenever the user maps change (save, delete, load).
    if (userMaps.revision >= 0) void converter.loadCatalog(userMaps.maps);
  });

  let outputName = $state('converted.mid');
  let draggingOver = $state(false);
  let dragUrl = $state('');

  const hintMapName = $derived(
    converter.maps.find((m) => m.id === converter.hint?.map_id)?.name ??
      converter.hint?.map_id ??
      '',
  );
  const coveragePercent = $derived(
    converter.coverage === null ? 100 : Math.round(converter.coverage * 100),
  );

  async function onDrop(event: DragEvent) {
    event.preventDefault();
    draggingOver = false;
    const file = event.dataTransfer?.files[0];
    if (file) await converter.loadFile(await readMidiFile(file));
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

  <nav class="views">
    <button
      class:active={view === 'convert'}
      onclick={() => (view = 'convert')}
      data-testid="view-convert"
    >
      Converter
    </button>
    <button class:active={view === 'maps'} onclick={() => (view = 'maps')} data-testid="view-maps">
      Map editor
    </button>
  </nav>

  {#if view === 'convert'}
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
            if (file) await converter.loadFile(await readMidiFile(file));
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
        Looks like {hintMapName}
        <button onclick={() => converter.setFromId(converter.hint!.map_id)}>Use as source</button>
      </p>
    {/if}

    <section class="picks">
      <label>
        Source map
        <select
          value={converter.fromId}
          onchange={(e) => converter.setFromId(e.currentTarget.value)}
        >
          {#each converter.maps as map (map.id)}
            <option value={map.id}>{map.name}</option>
          {/each}
        </select>
      </label>
      <label>
        Target map
        <select value={converter.toId} onchange={(e) => converter.setToId(e.currentTarget.value)}>
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
          targetNotes={converter.targetNotes}
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
  {:else}
    <MapEditorView {editor} />
  {/if}

  <footer>drum-map core v{version()}</footer>
</main>
