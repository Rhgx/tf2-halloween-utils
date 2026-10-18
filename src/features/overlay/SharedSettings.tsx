import { RotateCcw } from 'lucide-react';
import { useState } from 'react';

import { DEFAULT_OVERLAY_KEYS, type Overlay } from '../../api.ts';
import { Field, Group } from '../../components/fields.tsx';

export type KeySlot = keyof typeof DEFAULT_OVERLAY_KEYS;

type SharedSettingsProps = {
  overlay: Overlay;
  onMove: () => void;
  onReset: () => void;
  /** Resolves once the capture has ended, whether a key was taken, Esc cancelled it, or it failed (App toasts). */
  onCaptureKey: (which: KeySlot) => Promise<void>;
  onResetKey: (which: KeySlot) => void;
};

// One hotkey: the key field (click it, then press the key anywhere) and a reset to its default beside it.
function HotkeyRow({
  name,
  which,
  overlay,
  pending,
  onCapture,
  onReset
}: {
  name: string;
  which: KeySlot;
  overlay: Overlay;
  pending: KeySlot | null;
  onCapture: (which: KeySlot) => void;
  onReset: (which: KeySlot) => void;
}) {
  const key = which === 'toggle' ? overlay.settings.toggleKey : overlay.settings.editKey;
  const fallback = DEFAULT_OVERLAY_KEYS[which];
  const capturing = pending === which;
  return (
    <Field name={name}>
      <button
        type="button"
        className={capturing ? 'keyfield is-capturing' : 'keyfield'}
        aria-label={capturing ? 'Press a key. Esc cancels.' : `${name}: ${key.name}. Press to change.`}
        title={capturing ? 'Esc cancels' : undefined}
        disabled={pending !== null && !capturing}
        onClick={() => onCapture(which)}
      >
        {capturing ? 'Press a key' : key.name}
      </button>
      <button
        type="button"
        className="btn small icon"
        aria-label="Reset to the default key"
        title="Reset to the default key"
        disabled={pending !== null || key.code === fallback}
        onClick={() => onReset(which)}
      >
        <RotateCcw aria-hidden="true" />
      </button>
    </Field>
  );
}

// What every widget shares: the in-game layout and the hotkeys. Sits under the widget list.
export function SharedSettings({ overlay, onMove, onReset, onCaptureKey, onResetKey }: SharedSettingsProps) {
  const { settings, status } = overlay;
  // The hotkey being captured; one at a time, since the backend listens for a single key.
  const [pendingKey, setPendingKey] = useState<KeySlot | null>(null);

  function captureKey(which: KeySlot) {
    setPendingKey(which);
    onCaptureKey(which).finally(() => setPendingKey(null));
  }

  return (
    <div className="shared">
      <Group title="Layout">
        <Field
          wide
          name="Placed in TF2"
          help={`Drag a widget to move it, drag its corner to resize it, PgUp and PgDn fade it, ${settings.editKey.name} saves.`}
        >
          <button
            type="button"
            className="btn small"
            disabled={!settings.enabled || !status.tf2Running || status.editing}
            title={settings.enabled && !status.tf2Running ? 'Start TF2 first' : undefined}
            onClick={onMove}
          >
            Edit layout
          </button>
          <button type="button" className="btn small" onClick={onReset}>
            Reset layout
          </button>
        </Field>
      </Group>

      <Group title="Hotkeys">
        <HotkeyRow
          name="Hide and show"
          which="toggle"
          overlay={overlay}
          pending={pendingKey}
          onCapture={captureKey}
          onReset={onResetKey}
        />
        <HotkeyRow
          name="Edit layout"
          which="edit"
          overlay={overlay}
          pending={pendingKey}
          onCapture={captureKey}
          onReset={onResetKey}
        />
      </Group>
    </div>
  );
}
