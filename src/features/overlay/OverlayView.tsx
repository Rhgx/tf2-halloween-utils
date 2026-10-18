import { TriangleAlert } from 'lucide-react';
import { useState, type KeyboardEvent, type ReactNode } from 'react';

import type { GameState, Overlay, OverlaySettingsPatch } from '../../api.ts';
import { Switch } from '../../components/fields.tsx';
import type { TabStatus } from '../../components/TabNav.tsx';
import { CountdownPane } from './CountdownPane.tsx';
import { amount, endDate } from './panels.tsx';
import { RadarPane } from './RadarPane.tsx';
import { SharedSettings, type KeySlot } from './SharedSettings.tsx';

type OverlayViewProps = {
  className: string;
  overlay: Overlay;
  game: GameState | null;
  onEnable: (enabled: boolean) => void;
  onChange: (patch: OverlaySettingsPatch) => void;
  onMove: () => void;
  onReset: () => void;
  onCaptureKey: (which: KeySlot) => Promise<void>;
  onResetKey: (which: KeySlot) => void;
  onOpenSettings: () => void;
};

// How the radar widget is doing: what needs you, else how far the link (and the gargoyle) is.
export function radarLine({ settings, setupDone, status }: Overlay): TabStatus {
  if (!setupDone) return { text: 'Setup needed', tone: 'warn' };
  switch (status.link) {
    case 'unreachable':
      return { text: 'Unreachable', tone: 'warn' };
    case 'wrong-password':
      return { text: 'Wrong password', tone: 'warn' };
    case 'connected':
      return {
        text: status.reading ? `Gargoyle ${amount(status.reading.distHu, settings.radarUnits)}` : 'Connected',
        tone: 'live'
      };
    default:
      return { text: 'Waiting for TF2' };
  }
}

type Widget = 'radar' | 'countdown';

// One widget in the list: its name, and its switch on the right. The switch does not select.
function Node({
  id,
  name,
  selected,
  onSelect,
  children
}: {
  id: Widget;
  name: string;
  selected: boolean;
  onSelect: (id: Widget) => void;
  children: ReactNode;
}) {
  function onKeyDown(e: KeyboardEvent) {
    if (e.target !== e.currentTarget || (e.key !== 'Enter' && e.key !== ' ')) return;
    e.preventDefault();
    onSelect(id);
  }
  return (
    <li
      className="node"
      role="option"
      aria-selected={selected}
      tabIndex={0}
      onClick={() => onSelect(id)}
      onKeyDown={onKeyDown}
    >
      <span className="node-name">{name}</span>
      <span className="control" onClick={(e) => e.stopPropagation()}>
        {children}
      </span>
    </li>
  );
}

// The overlay tool. Left: the master switch, the widgets with their switches, and what they share. Right: the
// selected widget. Widgets are placed in game, in edit mode.
export function OverlayView({
  className,
  overlay,
  game,
  onEnable,
  onChange,
  onMove,
  onReset,
  onCaptureKey,
  onResetKey,
  onOpenSettings
}: OverlayViewProps) {
  const { settings } = overlay;
  const [selected, setSelected] = useState<Widget>('radar');
  const tf2Found = game?.tf2Found ?? true;

  return (
    <section id="view-overlay" className={className} role="tabpanel" aria-labelledby="tab-overlay">
      <main className="wrap overlay">
        {!tf2Found && (
          <div className="banner" role="status">
            <TriangleAlert aria-hidden="true" />
            <p>TF2 was not found in your Steam libraries, so the overlay can't follow it.</p>
            <button type="button" className="btn small" onClick={onOpenSettings}>
              Choose folder
            </button>
          </div>
        )}

        <div className="overlay-grid">
          <div>
            <div className="master">
              <span>Show in TF2</span>
              <Switch big label="Show in TF2" checked={settings.enabled} disabled={!tf2Found} onChange={onEnable} />
            </div>
            <ul className="list" role="listbox" aria-label="Widgets">
              <Node id="radar" name="Gargoyle radar" selected={selected === 'radar'} onSelect={setSelected}>
                <Switch
                  label="Gargoyle radar"
                  checked={settings.radar.enabled}
                  onChange={(enabled) => onChange({ radar: { enabled } })}
                />
              </Node>
              <Node
                id="countdown"
                name="Scream Fortress countdown"
                selected={selected === 'countdown'}
                onSelect={setSelected}
              >
                <Switch
                  label="Scream Fortress countdown"
                  checked={settings.countdown.enabled}
                  onChange={(enabled) => onChange({ countdown: { enabled } })}
                />
              </Node>
            </ul>
            <SharedSettings
              overlay={overlay}
              onMove={onMove}
              onReset={onReset}
              onCaptureKey={onCaptureKey}
              onResetKey={onResetKey}
            />
          </div>

          <div className="pane">
            {selected === 'radar' ? (
              <RadarPane overlay={overlay} onChange={onChange} />
            ) : (
              <CountdownPane overlay={overlay} end={endDate(game?.contractsEnd ?? null)} onChange={onChange} />
            )}
          </div>
        </div>
      </main>
    </section>
  );
}
