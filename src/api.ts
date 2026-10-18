// Talking to the Rust backend through Tauri commands. The types mirror the structs in src-tauri/src/commands.rs
// and game.rs.

import { invoke } from '@tauri-apps/api/core';

type LoginState = { status: 'waiting' | 'scanned' | 'failed'; qr: string; error?: string };

export type ServerState = {
  queued: string[];
  contracts: { completed: string[]; syncedAt: string } | null;
  manual: string[];
  loggedIn: boolean;
  // Always null until the Steam phase; the type keeps the sign-in dialog compiling until then.
  login: LoginState | null;
};

export type GameState = {
  tf2Found: boolean;
  running: boolean;
  logFound: boolean;
  queued: boolean;
  inMatch: boolean;
  map: string | null;
  played: { code: string; at: number }[];
  autoSyncError: string | null;
  syncing: boolean;
  syncedAt: string | null;
  /** When this year's Scream Fortress contracts end (YYYY-MM-DD), read from TF2's files. */
  contractsEnd: string | null;
};

export type Settings = {
  closeToTray: boolean;
  startMinimized: boolean;
  startWithWindows: boolean;
  restoreQueueOnQuit: boolean;
  tf2Folder: string | null;
  tf2FolderDetected: string | null;
  tf2FolderOverridden: boolean;
  dataFolder: string;
  version: string;
  platform: 'windows' | 'linux' | 'other';
};

export type SettingsPatch = Partial<
  Pick<Settings, 'closeToTray' | 'startMinimized' | 'startWithWindows' | 'restoreQueueOnQuit'>
>;

type RadarLink = 'off' | 'waiting' | 'unreachable' | 'wrong-password' | 'connected';

type OverlayEvent = { at: number; text: string; tone: 'info' | 'good' | 'warn' | 'bad' | 'gargoyle' };

/** Distance and height in hammer units, TF2's own (0.75 inch); height is relative to your feet, positive above. */
export type RadarReading = { angleDeg: number; distHu: number; heightHu: number };

type OverlayKey = { code: number; name: string };

/** The default hotkeys' virtual-key codes (src-tauri/src/overlay/settings.rs): F3 and F4. */
export const DEFAULT_OVERLAY_KEYS: Record<'toggle' | 'edit', number> = { toggle: 0x72, edit: 0x73 };

/** One overlay widget: its switch, its centre as fractions of TF2's client area, and its look. */
export type OverlayWidget = { enabled: boolean; posX: number; posY: number; opacity: number; scale: number };

/** How the countdown writes the time left (src-tauri/src/overlay/widgets/countdown.rs). */
export type CountdownFormat = 'adaptive' | 'days' | 'detailed' | 'clock';

/** The unit the radar writes distances in (src-tauri/src/overlay/widgets/radar/nav.rs). */
export type RadarUnits = 'metres' | 'feet' | 'hammer';

export type Overlay = {
  settings: {
    /** The master switch: off hides every widget and stops the overlay. */
    enabled: boolean;
    radar: OverlayWidget;
    countdown: OverlayWidget;
    countdownFormat: CountdownFormat;
    radarUnits: RadarUnits;
    port: number;
    demoDelayS: number;
    anyGargoyle: boolean;
    /** The radar panel shows how long until the gargoyle despawns. */
    despawnTimer: boolean;
    /** In-game hotkeys: a Windows virtual-key code and its readable name. */
    toggleKey: OverlayKey;
    editKey: OverlayKey;
  };
  /** The full launch options line to copy, password and -condebug included. */
  launchOptions: string;
  /** RCON has connected at least once with these options. */
  setupDone: boolean;
  status: {
    tf2Running: boolean;
    /** 'off' while the radar is disabled. */
    link: RadarLink;
    /** getpos replies per second while reading. */
    positionHz: number | null;
    /** The recording being followed; intervalS is null until the write interval is measured. */
    demo: { file: string; intervalS: number | null; fillerKbps: number } | null;
    /** leftS: seconds until it despawns; null if it spawned before the recording started. */
    gargoyle: { map: string | null; x: number; y: number; z: number; leftS: number | null } | null;
    /** The live arrow, once a gargoyle and a pose exist. */
    reading: RadarReading | null;
    /** Overlay edit mode is on, or pending until TF2 is in front. */
    editing: boolean;
  };
  /** Newest first, at most 8. */
  events: OverlayEvent[];
};

/** A widget's switch and look; its position changes in edit mode only. */
export type OverlayWidgetPatch = Partial<Pick<OverlayWidget, 'enabled' | 'opacity' | 'scale'>>;

export type OverlaySettingsPatch = Partial<
  Pick<
    Overlay['settings'],
    'countdownFormat' | 'radarUnits' | 'port' | 'demoDelayS' | 'anyGargoyle' | 'despawnTimer'
  > & {
    radar: OverlayWidgetPatch;
    countdown: OverlayWidgetPatch;
  }
>;

export function getState() {
  return invoke<ServerState>('get_state');
}

export function getSettings() {
  return invoke<Settings>('get_settings');
}

export function setSettings(patch: SettingsPatch) {
  return invoke<Settings>('set_settings', { patch });
}

// Opens the app's folder picker; resolves with the settings as they are afterwards, also when cancelled.
export function chooseTf2Folder() {
  return invoke<Settings>('choose_tf2_folder');
}

export function useDetectedTf2Folder() {
  return invoke<Settings>('use_detected_tf2_folder');
}

export function openDataFolder() {
  return invoke<void>('open_data_folder');
}

export function getGame() {
  return invoke<GameState>('get_game');
}

export function setQueue(codes: string[]) {
  return invoke<ServerState>('set_queue', { codes });
}

export function setManual(codes: string[]) {
  return invoke<ServerState>('set_manual', { codes });
}

export function launchTf2() {
  return invoke<GameState>('launch_tf2');
}

export function openLink(url: string) {
  return invoke<void>('open_link', { url });
}

export function quit() {
  return invoke<void>('quit');
}

// Null on platforms without the overlay (Linux).
export function getOverlay() {
  return invoke<Overlay | null>('get_overlay');
}

// The master switch. Both take a moment when they stop the overlay's threads.
export function setOverlayEnabled(enabled: boolean) {
  return invoke<Overlay>('set_overlay_enabled', { enabled });
}

export function setOverlaySettings(patch: OverlaySettingsPatch) {
  return invoke<Overlay>('set_overlay_settings', { patch });
}

// Enters edit mode for every widget; applies when TF2 is next in front.
export function editOverlayLayout() {
  return invoke<Overlay>('edit_overlay_layout');
}

// Puts every widget's position, the scale and the opacity back to their defaults.
export function resetOverlayLayout() {
  return invoke<Overlay>('reset_overlay_layout');
}

// Waits up to 10 s for the next key or mouse button pressed anywhere (captured natively, so the window needs no
// focus). Esc cancels and resolves unchanged; a timeout, a key edit mode uses, or the other hotkey's key rejects.
export function captureOverlayKey(which: 'toggle' | 'edit') {
  return invoke<Overlay>('capture_overlay_key', { which });
}

// Puts one hotkey back to its default; if the other hotkey sits on that key, the backend moves it to this one's
// old key so they never collide.
export function resetOverlayKey(which: 'toggle' | 'edit') {
  return invoke<Overlay>('reset_overlay_key', { which });
}

// Steam: QR sign-in (the dialog polls getState while it is pending), sign-out and contract sync.
export function startLogin() {
  return invoke<ServerState>('start_login');
}

export function cancelLogin() {
  return invoke<ServerState>('cancel_login');
}

export function logout() {
  return invoke<ServerState>('logout');
}

export function syncContracts() {
  return invoke<ServerState>('sync');
}

// Tauri rejects with the Rust error string as is; Error objects still give their message.
export function errorMessage(err: unknown) {
  if (typeof err === 'string') return err;
  return err instanceof Error ? err.message : String(err);
}
