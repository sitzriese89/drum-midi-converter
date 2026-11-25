// The web build's file handling (M4): no Tauri, so the picker button opens the browser file
// input, the converted file leaves as a download, and there is no drag-out link. Runs the
// whole App against the real WASM core (test-setup.ts) in jsdom, which has no Tauri globals.
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';

/** Format-0 SMF: one kick hit (36) plus a cowbell (56) on channel 10. */
function drumMidi(): Uint8Array {
  return new Uint8Array([
    0x4d, 0x54, 0x68, 0x64, 0, 0, 0, 6, 0, 0, 0, 1, 0x01, 0xf4, 0x4d, 0x54, 0x72, 0x6b, 0, 0, 0, 15,
    0, 0x99, 36, 100, 0x60, 0x89, 36, 64, 0, 0x99, 56, 80, 0x60, 0x89, 56, 64, 0x60, 0xff, 0x2f,
    0x00,
  ]);
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('App (web build)', () => {
  it('the picker button opens the browser file input', async () => {
    render(App);
    const input = screen.getByLabelText('Choose MIDI file') as HTMLInputElement;
    const click = vi.spyOn(input, 'click').mockImplementation(() => {});
    await fireEvent.click(screen.getByTestId('pick-file'));
    expect(click).toHaveBeenCalledOnce();
  });

  it('a picked file converts and downloads, with no drag-out link', async () => {
    const createObjectURL = vi.fn(() => 'blob:converted');
    vi.stubGlobal('URL', Object.assign(URL, { createObjectURL, revokeObjectURL: vi.fn() }));
    const anchorClick = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
    render(App);
    // Wait for the catalog to fill the dropdowns.
    await waitFor(() =>
      expect(screen.getByLabelText('Source map').querySelectorAll('option').length).toBeGreaterThan(
        1,
      ),
    );

    const input = screen.getByLabelText('Choose MIDI file') as HTMLInputElement;
    const file = new File([drumMidi() as BlobPart], 'groove.mid', { type: 'audio/midi' });
    Object.defineProperty(input, 'files', { value: [file], configurable: true });
    await fireEvent.change(input);
    await waitFor(() => expect(screen.getByText('groove.mid')).toBeInTheDocument());

    await fireEvent.click(screen.getByRole('button', { name: 'Convert' }));
    const save = await screen.findByTestId('save');
    expect(save).toHaveTextContent('Download converted file');
    expect(screen.queryByTestId('drag-out')).toBeNull();

    await fireEvent.click(save);
    await waitFor(() => expect(anchorClick).toHaveBeenCalledOnce());
    const anchor = anchorClick.mock.contexts[0] as HTMLAnchorElement;
    expect(anchor.download).toMatch(/^groove \(.+\)\.mid$/);
    expect(anchor.href).toBe('blob:converted');
  });
});
