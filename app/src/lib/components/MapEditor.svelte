<script lang="ts">
  // The map editor screen (SPEC §3.4, Q6): list user maps, create/duplicate/edit notes,
  // import/export shared TOML. Built-ins stay read-only — "Duplicate" loads a copy.
  import type { MapEditor } from '../map-editor.svelte';
  import type { MapIssue } from '../drum-map';

  let { editor }: { editor: MapEditor } = $props();

  /** The issues concerning one note row, by note number: the core keys
   * `notes.<n>…` fields by MIDI note number (not row index), so they follow
   * the row when notes are added or removed. */
  function noteIssues(note: number, suffix: string): MapIssue[] {
    return editor.issues.filter((issue) => issue.field === `notes.${note}.${suffix}`);
  }

  /** The issues not attached to a note row (id, name, vendor, toml, …). */
  function otherIssues(): MapIssue[] {
    return editor.issues.filter((issue) => !issue.field.startsWith('notes.'));
  }

  async function onImportFile(event: Event) {
    const input = event.currentTarget;
    if (!(input instanceof HTMLInputElement)) return;
    const file = input.files?.[0];
    if (!file) return;
    editor.importToml(await file.text(), file.name);
    input.value = '';
  }

  async function onExport() {
    await editor.exportMap();
  }
</script>

<section class="map-editor" data-testid="map-editor">
  <h2>Map editor</h2>

  {#if !editor.userMaps.available}
    <p class="warning" role="alert" data-testid="unavailable-note">
      User maps are only available in the desktop app (they are stored in the app-data folder).
      Editing, saving and importing are disabled here.
    </p>
  {/if}

  {#if editor.error}
    <p class="error" role="alert">{editor.error}</p>
  {/if}
  {#if editor.status}
    <p class="status" data-testid="editor-status">{editor.status}</p>
  {/if}

  <div class="columns">
    <div class="library">
      <h3>Your maps</h3>
      <ul data-testid="user-map-list">
        {#each editor.userMaps.maps as map (map.id)}
          <li>
            <button disabled={!editor.userMaps.available} onclick={() => editor.editStored(map.id)}>
              Edit
            </button>
            <button disabled={!editor.userMaps.available} onclick={() => editor.deleteMap(map.id)}>
              Delete
            </button>
            {map.name} <span class="map-id">({map.id})</span>
          </li>
        {:else}
          <li>No user maps yet.</li>
        {/each}
      </ul>

      <h3>Built-in maps (read-only)</h3>
      <ul data-testid="builtin-map-list">
        {#each editor.builtins as map (map.id)}
          <li>
            <button
              disabled={!editor.userMaps.available}
              onclick={() => editor.duplicateBuiltin(map.id)}
            >
              Duplicate
            </button>
            {map.name}
          </li>
        {/each}
      </ul>

      <p>
        <button
          disabled={!editor.userMaps.available}
          onclick={() => editor.newMap()}
          data-testid="new-map"
        >
          New map
        </button>
        <label class="import">
          Import TOML
          <input
            type="file"
            accept=".toml,text/plain"
            aria-label="Import map TOML"
            disabled={!editor.userMaps.available}
            onchange={onImportFile}
          />
        </label>
        <button disabled={!editor.userMaps.available} onclick={onExport} data-testid="export">
          Export…
        </button>
      </p>
    </div>

    <div class="draft" data-testid="map-draft">
      {#if editor.draft}
        <h3>Draft</h3>
        <p class="meta">
          <label>
            Id
            <input
              bind:value={editor.draft.id}
              disabled={!editor.userMaps.available}
              data-testid="draft-id"
            />
          </label>
          <label>
            Name
            <input bind:value={editor.draft.name} disabled={!editor.userMaps.available} />
          </label>
          <label>
            Vendor
            <input bind:value={editor.draft.vendor} disabled={!editor.userMaps.available} />
          </label>
          <label>
            Description
            <input
              value={editor.draft.description ?? ''}
              onchange={(e) => (editor.draft!.description = e.currentTarget.value || null)}
              disabled={!editor.userMaps.available}
            />
          </label>
        </p>

        <table class="notes" data-testid="note-table">
          <thead>
            <tr>
              <th scope="col">Note</th>
              <th scope="col">Instrument</th>
              <th scope="col">Tag</th>
              <th scope="col"></th>
            </tr>
          </thead>
          <tbody>
            {#each editor.draft.notes as note (note.note)}
              <tr>
                <td>
                  <input
                    type="number"
                    min="0"
                    max="127"
                    bind:value={note.note}
                    disabled={!editor.userMaps.available}
                    aria-label="MIDI note number"
                  />
                  {#each noteIssues(note.note, 'note') as issue, i (i)}
                    <span class="issue" role="alert">{issue.message}</span>
                  {/each}
                </td>
                <td>
                  <input bind:value={note.name} disabled={!editor.userMaps.available} />
                  {#each noteIssues(note.note, 'name') as issue, i (i)}
                    <span class="issue" role="alert">{issue.message}</span>
                  {/each}
                </td>
                <td>
                  <input bind:value={note.tag} disabled={!editor.userMaps.available} />
                  {#each noteIssues(note.note, 'tag') as issue, i (i)}
                    <span class="issue" role="alert">{issue.message}</span>
                  {/each}
                </td>
                <td>
                  <button
                    disabled={!editor.userMaps.available}
                    onclick={() => editor.removeNote(note.note)}
                    aria-label={`Delete note ${note.note}`}
                  >
                    ✕
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        <p>
          <button
            disabled={!editor.userMaps.available}
            onclick={() => editor.addNote()}
            data-testid="add-note"
          >
            Add note
          </button>
        </p>

        {#if editor.draft.fallback}
          <h4>Fallback rules</h4>
          <ul class="fallback">
            {#each editor.draft.fallback as rule, index (index)}
              <li>
                <input bind:value={rule.from} disabled={!editor.userMaps.available} />
                →
                <input bind:value={rule.to} disabled={!editor.userMaps.available} />
                <button
                  disabled={!editor.userMaps.available}
                  onclick={() => editor.removeFallbackRule(index)}
                >
                  ✕
                </button>
              </li>
            {/each}
          </ul>
        {/if}
        <p>
          <button disabled={!editor.userMaps.available} onclick={() => editor.addFallbackRule()}>
            Add fallback rule
          </button>
        </p>

        {#if editor.issues.length > 0}
          <div class="issues" role="alert" data-testid="validation-issues">
            {#each otherIssues() as issue (issue.field + issue.message)}
              <p><strong>{issue.field}:</strong> {issue.message}</p>
            {/each}
          </div>
        {/if}

        <p>
          <button
            disabled={!editor.userMaps.available}
            onclick={() => editor.save()}
            data-testid="save-map"
          >
            Save
          </button>
        </p>
      {:else}
        <p class="status">Create, duplicate or import a map to start editing.</p>
      {/if}
    </div>
  </div>
</section>
