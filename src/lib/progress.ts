import { get, writable } from "svelte/store";
import { tick } from "svelte";
import { invoke } from "@tauri-apps/api/core";

export const isRunning = writable(false);
export const terminalLines = writable<string[]>([]);
// Whether the terminal output box is expanded. Lives here (not in
// TerminalOutput.svelte) so the toggle button can sit next to a view's own
// action buttons instead of being pinned under the terminal box itself.
export const terminalExpanded = writable(false);
// Real progress fraction (0-1) parsed from the command's own output (e.g.
// dpkg's "Leyendo la base de datos ... 45%"), or null when the current
// output gives no percentage to go on — the bar falls back to an
// indeterminate animation in that case instead of pretending to know.
export const progressFraction = writable<number | null>(null);

interface JobStatus {
  running: boolean;
  success: boolean;
  output: string;
  waitingConfirmation: boolean;
}

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// Only lines that actually look like a progress bar: wget's "  45%[...]"
// (percent near the start of the line) or dpkg/apt/flatpak's
// "Progress: [ 45%]" / "Progreso: [ 45%]". A bare "N%" anywhere else in the
// output (e.g. an unrelated "CPU usage: 12%" line a script might print) is
// deliberately NOT matched — that used to make the bar jump around or look
// stalled on output that had nothing to do with actual progress.
const PROGRESS_LINE_PATTERN = /(?:^\s*(\d{1,3})%|[Pp]rogres[so]:?\s*\[\s*(\d{1,3})%\s*\])/;

// apt's own end-of-resolution summary line, across the formats seen in the
// wild: modern apt's localized "Resumen: Actualizando: 0, Instalando: 2,
// Eliminando: 1, no actualizando: 20" and the classic English
// "2 upgraded, 3 newly installed, 1 to remove". Each captures the three
// package counts regardless of which of the two forms apt printed.
const SUMMARY_PATTERNS = [
  /Actualizando:\s*(\d+),\s*Instalando:?\s*(\d+),\s*Eliminando:\s*(\d+)/i,
  /(\d+)\s+upgraded,\s+(\d+)\s+newly installed,\s+(\d+)\s+to remove/i,
];

// Lines dpkg prints once per package while it unpacks/configures it — used
// as the numerator against the total from the summary line above.
const PACKAGE_STEP_PATTERN =
  /^(Unpacking|Desempaquetando|Preparando para desempaquetar|Setting up|Configurando)\b/;

function estimateFraction(output: string): number | null {
  const lines = output.split("\n");
  for (let i = lines.length - 1; i >= 0; i--) {
    const match = lines[i].match(PROGRESS_LINE_PATTERN);
    if (match) {
      const value = Number(match[1] ?? match[2]);
      return Math.min(100, Math.max(0, value)) / 100;
    }
  }

  let total = 0;
  for (const pattern of SUMMARY_PATTERNS) {
    const match = output.match(pattern);
    if (match) {
      total = Number(match[1]) + Number(match[2]) + Number(match[3]);
      break;
    }
  }
  if (total <= 0) {
    return null;
  }

  const steps = output.split("\n").filter((line) => PACKAGE_STEP_PATTERN.test(line.trim())).length;
  // Each package produces roughly two steps (unpack + configure, or the
  // Desempaquetando/Preparando pair on removal), so divide by total*2.
  return Math.min(1, steps / (total * 2));
}

// Scrolls the terminal panel into view the first time it actually has
// content to show. It can't run at job start (terminalExpanded=true isn't
// enough — the panel is only mounted once terminalLines.length > 0, so the
// element doesn't exist yet), and afterUpdate-based edge detection inside
// TerminalOutput.svelte missed jobs fast enough to flip isRunning back to
// false before a render tick observed it running — instant local AppImage
// installs with no apt/download progress lines are exactly that case.
let scrolledForCurrentJob = false;

async function scrollToTerminal() {
  await tick();
  requestAnimationFrame(() => {
    const el = document.getElementById("job-terminal-anchor");
    const container = document.getElementById("main-scroll");
    if (el && container) {
      const targetTop = el.getBoundingClientRect().top - container.getBoundingClientRect().top + container.scrollTop;
      // No "smooth": WebKitGTK's animated scrollTo can crash the WebProcess
      // on VM software rendering (same class of issue as the DMA-BUF
      // renderer workaround in main.rs).
      container.scrollTo({ top: targetTop });
    }
  });
}

function applyOutput(output: string, base: string[] = []) {
  if (!output) {
    return;
  }
  const lines = output.split("\n").filter((line) => line.trim().length > 0);
  if (!scrolledForCurrentJob && lines.length > 0) {
    scrolledForCurrentJob = true;
    scrollToTerminal();
  }
  terminalLines.set([...base, ...lines]);
  progressFraction.set(estimateFraction(output));
}

// Starts the job (a fast call that only spawns a background thread in Rust
// and returns immediately) then polls for its status every 300ms — each
// poll is itself a fast, non-blocking lookup. The backend streams stdout/stderr
// into the job's output line by line as the process runs, so each poll can
// show live progress instead of only revealing output once the job is done.
// `append` keeps whatever the previous job left in the terminal and adds this
// job's output below it, so a multi-part update (apt/snap/flatpak-system as
// root, then user Flatpaks unprivileged) reads as one continuous log.
export async function runTracked(
  command: string,
  args?: Record<string, unknown>,
  options?: { append?: boolean },
): Promise<boolean> {
  // Only one job at a time: isRunning/terminalLines/progressFraction are a
  // single shared store, not one per job, so a second job running now would
  // reset and interleave with whatever the first one is still writing to
  // the screen. Refusing to start keeps that shared state meaningful instead
  // of redesigning it to be per-job.
  if (get(isRunning)) return false;
  const base = options?.append ? get(terminalLines) : [];
  scrolledForCurrentJob = !!options?.append;
  terminalLines.set(base);
  progressFraction.set(null);
  isRunning.set(true);
  terminalExpanded.set(true);
  try {
    const jobId = await invoke<number>(command, args);

    while (true) {
      await sleep(300);
      const status = await invoke<JobStatus>("poll_job", { jobId });
      applyOutput(status.output, base);
      if (!status.running) {
        return status.success;
      }
    }
  } catch (error) {
    terminalLines.set([String(error)]);
    return false;
  } finally {
    isRunning.set(false);
    progressFraction.set(null);
  }
}

// Like runTracked, but for a job started with start_update_session(): it
// pauses partway through (after `apt update`) waiting for confirmation.
// `onWaitingForConfirmation` is called at that point (to show the "N updates
// available" popup) and must resolve to what the user picked, or null to
// skip the follow-up — sent back via confirm_pending_job so the SAME
// already-authenticated pkexec process runs it (or doesn't) without a second
// password prompt either way.
//
// What the user ticked for the root-side session; the follow-up command is
// built and validated in Rust (confirm_pending_job), never sent as text.
export interface UpgradeSelection {
  aptAll: boolean;
  apt: string[];
  snap: string[];
  flatpakSystem: string[];
}

export async function runConfirmableUpdate(
  command: string,
  onWaitingForConfirmation: () => Promise<UpgradeSelection | null>,
): Promise<boolean> {
  if (get(isRunning)) return false;
  scrolledForCurrentJob = false;
  terminalLines.set([]);
  progressFraction.set(null);
  isRunning.set(true);
  terminalExpanded.set(true);
  try {
    const jobId = await invoke<number>(command);
    let confirmed = false;

    while (true) {
      await sleep(300);
      const status = await invoke<JobStatus>("poll_job", { jobId });
      applyOutput(status.output);
      if (status.waitingConfirmation && !confirmed) {
        confirmed = true;
        const selection = await onWaitingForConfirmation();
        await invoke("confirm_pending_job", { jobId, selection });
      }
      if (!status.running) {
        return status.success;
      }
    }
  } catch (error) {
    terminalLines.set([String(error)]);
    return false;
  } finally {
    isRunning.set(false);
    progressFraction.set(null);
  }
}
