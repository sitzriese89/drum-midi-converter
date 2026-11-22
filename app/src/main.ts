import { mount } from 'svelte';
import App from './App.svelte';
import { initDrumMap } from './lib/drum-map';
import './styles/base.css';

// The drum-map core must be loaded before any component calls it.
initDrumMap().then(() => {
  mount(App, { target: document.getElementById('app')! });
});
