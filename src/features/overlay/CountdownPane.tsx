import type { Overlay, OverlaySettingsPatch } from '../../api.ts';
import { COUNTDOWN_FORMATS, CountdownPanel, countdownText, Preview, SAMPLE_END, SAMPLE_LEFT_S } from './panels.tsx';

type CountdownPaneProps = {
  overlay: Overlay;
  /** When the contracts end, from TF2's items_game.txt; null when TF2's files give none. */
  end: Date | null;
  onChange: (patch: OverlaySettingsPatch) => void;
};

// The Scream Fortress countdown widget: its preview, when it ends, and how it writes the time left.
export function CountdownPane({ overlay, end, onChange }: CountdownPaneProps) {
  const { settings } = overlay;
  // Without an end date the sample stands in, as it does in edit mode in game.
  const leftS = end ? Math.floor((end.getTime() - Date.now()) / 1000) : SAMPLE_LEFT_S;
  const countdown = countdownText(leftS, end ?? SAMPLE_END, settings.countdownFormat);
  return (
    <div className="pane-now">
      <Preview
        widget={settings.countdown}
        cap={end ? (countdown ? undefined : 'Hidden, a day after the end') : 'Sample'}
        onChange={(countdown) => onChange({ countdown })}
      >
        {countdown && <CountdownPanel countdown={countdown} />}
      </Preview>

      <dl className="rows">
        <dt>Ends</dt>
        <dd className={end ? undefined : 'idle'}>
          {end ? (
            <>
              {end.toLocaleString('en-GB', { dateStyle: 'full', timeStyle: 'short' })}
              <small>· 00:00 UTC on the day in TF2's items_game.txt</small>
            </>
          ) : (
            "No end date in TF2's files"
          )}
        </dd>
        <dt>Format</dt>
        <dd className="choices" role="radiogroup" aria-label="Countdown format">
          {COUNTDOWN_FORMATS.map(({ value, label }) => (
            <label key={value}>
              <input
                type="radio"
                name="countdownFormat"
                value={value}
                checked={settings.countdownFormat === value}
                onChange={() => onChange({ countdownFormat: value })}
              />
              {label}
              <small>· {countdownText(Math.max(leftS, 1), end ?? SAMPLE_END, value)?.big}</small>
            </label>
          ))}
        </dd>
      </dl>
    </div>
  );
}
