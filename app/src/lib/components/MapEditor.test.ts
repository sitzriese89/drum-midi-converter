// The map editor screen (SPEC §3.4): the lists, the draft form, validation errors in the
// UI, and the user map appearing in the list after save. Runs against the real WASM core
// (loaded in test-setup.ts) with an in-memory user-map store.
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import MapEditorView from './MapEditor.svelte';
import { MapEditor } from '../map-editor.svelte';
import { UserMaps, type MapStorage } from '../user-maps.svelte';
import { listMaps } from '../drum-map';

function memoryStorage(): MapStorage {
  const files = new Map<string, string>();
  return {
    available: true,
    list: async () => [...files.keys()].sort(),
    read: async (id) => files.get(id) ?? null,
    write: async (id, content) => {
      files.set(id, content);
    },
    delete: async (id) => {
      files.delete(id);
    },
    exportFile: async () => null,
  };
}

function unavailableStorage(): MapStorage {
  return { ...memoryStorage(), available: false };
}

async function renderEditor(storage: MapStorage = memoryStorage()) {
  const userMaps = new UserMaps(storage);
  const editor = new MapEditor(userMaps);
  await editor.load(await listMaps());
  render(MapEditorView, { editor });
  return editor;
}

describe('MapEditor (component)', () => {
  it('lists the built-in maps as read-only duplicate sources', async () => {
    await renderEditor();
    const builtins = screen.getByTestId('builtin-map-list');
    expect(builtins).toHaveTextContent('General MIDI');
    // No Edit/Delete for built-ins, only Duplicate.
    expect(screen.queryByText('Edit')).toBeNull();
    expect(screen.getAllByRole('button', { name: 'Duplicate' }).length).toBeGreaterThanOrEqual(7);
  });

  it('duplicating a built-in opens the draft with its notes', async () => {
    await renderEditor();
    fireEvent.click(screen.getAllByRole('button', { name: 'Duplicate' })[0]);
    await waitFor(() => expect(screen.getByTestId('draft-id')).toHaveProperty('value', 'gm-copy'));
    // Input values, not text content: the note rows are form fields.
    expect(screen.getByDisplayValue('Bass Drum 1')).toBeInTheDocument();
    expect(screen.getByDisplayValue('36')).toBeInTheDocument();
  });

  it('shows validation errors in the UI and never saves an invalid draft', async () => {
    const editor = await renderEditor();
    fireEvent.click(screen.getByTestId('new-map'));
    await waitFor(() => expect(screen.getByTestId('draft-id')).toBeInTheDocument());
    // Break the tag: the error appears next to the note row and save is refused.
    editor.draft!.notes[0].tag = 'Kick!';
    fireEvent.click(screen.getByTestId('save-map'));
    await waitFor(() => expect(screen.getByTestId('validation-issues')).toBeInTheDocument());
    expect(screen.getByTestId('validation-issues')).toHaveTextContent(/lowercase/);
    expect(screen.getByTestId('user-map-list')).toHaveTextContent('No user maps yet');
  });

  it('shows a per-note issue next to its row (fields are keyed by note number)', async () => {
    const editor = await renderEditor();
    fireEvent.click(screen.getByTestId('new-map'));
    await waitFor(() => expect(screen.getByTestId('draft-id')).toBeInTheDocument());
    // A bad tag on note 36: the core reports `notes.36.tag`, and the row shows it.
    editor.draft!.notes[0].tag = 'Kick!';
    fireEvent.click(screen.getByTestId('save-map'));
    await waitFor(() => expect(screen.getByTestId('validation-issues')).toBeInTheDocument());
    const row = screen.getByLabelText('MIDI note number').closest('tr');
    expect(row).not.toBeNull();
    expect(row!).toHaveTextContent(/lowercase/);
  });

  it('a saved user map appears in the list', async () => {
    const editor = await renderEditor();
    fireEvent.click(screen.getByTestId('new-map'));
    await waitFor(() => expect(screen.getByTestId('draft-id')).toBeInTheDocument());
    fireEvent.input(screen.getByTestId('draft-id'), { target: { value: 'my-kit' } });
    editor.draft!.name = 'My Kit';
    editor.draft!.vendor = 'Me';
    fireEvent.click(screen.getByTestId('save-map'));
    await waitFor(() =>
      expect(screen.getByTestId('editor-status')).toHaveTextContent(/"my-kit" saved/),
    );
    expect(screen.getByTestId('user-map-list')).toHaveTextContent('my-kit');
  });

  it('shows where maps are kept when the storage has a caveat (web build)', async () => {
    await renderEditor({ ...memoryStorage(), notice: 'Maps are saved in this browser only.' });
    expect(screen.getByTestId('storage-notice')).toHaveTextContent(/this browser only/);
    expect(screen.queryByTestId('unavailable-note')).toBeNull();
    expect(screen.getByTestId('new-map')).toBeEnabled();
  });

  it('is disabled with a note when user maps are unavailable (browser storage blocked)', async () => {
    await renderEditor(unavailableStorage());
    expect(screen.getByTestId('unavailable-note')).toHaveTextContent(/blocks site storage/);
    expect(screen.getByTestId('new-map')).toBeDisabled();
    expect(screen.getByTestId('export')).toBeDisabled();
    // The built-in Duplicate buttons are disabled too: there is nowhere to save the copy.
    for (const button of screen.getAllByRole('button', { name: 'Duplicate' })) {
      expect(button).toBeDisabled();
    }
  });
});
