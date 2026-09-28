import { writable } from "svelte/store";

export type Theme = "light" | "dark";

const STORAGE_KEY = "soplos-welcome-theme";

function readStoredTheme(): Theme {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored === "light" ? "light" : "dark";
  } catch {
    return "dark";
  }
}

function applyTheme(value: Theme) {
  document.documentElement.classList.toggle("light", value === "light");
  try {
    localStorage.setItem(STORAGE_KEY, value);
  } catch {
    // Private/blocked storage — the toggle still works for this session.
  }
}

// Read the saved preference BEFORE creating the store. subscribe() below
// fires synchronously with the store's initial value, and applyTheme also
// writes to localStorage — creating the store as "dark" and reading storage
// afterwards meant that first write clobbered the saved preference with
// "dark" on every single launch, before it was ever read back.
export const theme = writable<Theme>(readStoredTheme());

// Applies whatever the store holds to <html> on every change, including
// the initial value set right after creation above.
theme.subscribe(applyTheme);

export function toggleTheme() {
  theme.update((current) => (current === "dark" ? "light" : "dark"));
}
