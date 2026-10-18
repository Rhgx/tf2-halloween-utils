import { Check, Copy, TriangleAlert } from 'lucide-react';
import { useEffect, useState, type ReactNode } from 'react';

import type { Overlay, OverlaySettingsPatch } from '../../api.ts';
import { Field, Group, Slider, Switch } from '../../components/fields.tsx';
import { clock, Preview, RADAR_UNITS, RadarPanel, SAMPLE_READING } from './panels.tsx';

type RadarPaneProps = {
  overlay: Overlay;
  onChange: (patch: OverlaySettingsPatch) => void;
};

const time = (at: number) => new Date(at).toLocaleTimeString('en-GB');

function Row({ label, tone, children }: { label: string; tone?: 'idle' | 'bad'; children: ReactNode }) {
  return (
    <>
      <dt>{label}</dt>
      <dd className={tone}>
        {tone === 'bad' && <TriangleAlert aria-hidden="true" />}
        {children}
      </dd>
    </>
  );
}

function CopyLine({ text }: { text: string }) {
  const [state, setState] = useState<'idle' | 'copied' | 'failed'>('idle');
  useEffect(() => {
    if (state === 'idle') return;
    const timer = setTimeout(() => setState('idle'), 1200);
    return () => clearTimeout(timer);
  }, [state]);
  return (
    <div className="copy">
      <code>{text}</code>
      <button
        type="button"
        onClick={() =>
          navigator.clipboard.writeText(text).then(
            () => setState('copied'),
            () => setState('failed')
          )
        }
      >
        {state === 'copied' ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
        {state === 'idle' ? 'Copy' : state === 'copied' ? 'Copied' : "Couldn't copy"}
      </button>
    </div>
  );
}

// The Gargoyle radar widget. Row 1 is now (the preview beside the live status, or the setup steps on first run),
// row 2 is how the radar behaves.
export function RadarPane({ overlay, onChange }: RadarPaneProps) {
  const { settings, status, setupDone } = overlay;
  const [portDraft, setPortDraft] = useState<string | null>(null);
  const [showSetup, setShowSetup] = useState(false);
  const setup = !setupDone || showSetup;

  // The steps fold into the live rows as soon as RCON first connects, also when they were reopened.
  useEffect(() => setShowSetup(false), [setupDone]);

  const reading = status.reading ?? SAMPLE_READING;
  const connected = status.link === 'connected';
  const on = settings.enabled && settings.radar.enabled;

  function savePort() {
    const port = Number(portDraft);
    setPortDraft(null);
    if (!Number.isInteger(port) || port < 1024 || port > 65535 || port === settings.port) return;
    onChange({ port });
  }

  // One row for the link to TF2: whether the game runs and whether RCON answers are one story.
  const link =
    status.link === 'connected'
      ? `Connected on port ${settings.port}`
      : status.link === 'unreachable'
        ? 'Refused: TF2 is running without the launch options'
        : status.link === 'wrong-password'
          ? 'Wrong password: restart TF2 with the current launch options'
          : status.link === 'off'
            ? 'Off'
            : status.tf2Running
              ? 'Connecting'
              : 'TF2 not running';
  const linkBad = status.link === 'unreachable' || status.link === 'wrong-password';
  const linkIdle = status.link === 'off' || (status.link === 'waiting' && !status.tf2Running);

  return (
    <>
      <div className="pane-now">
        <Preview
          widget={settings.radar}
          cap={status.reading ? undefined : 'Sample'}
          onChange={(radar) => onChange({ radar })}
        >
          <RadarPanel
            reading={reading}
            units={settings.radarUnits}
            leftS={status.reading ? (status.gargoyle?.leftS ?? null) : settings.despawnTimer ? 192 : null}
          />
        </Preview>

        {setup ? (
          <div className="setup">
            <p className="kicker">Set up once</p>
            <h3>Add the launch options to TF2</h3>
            <ol className="steps">
              <li>
                <strong>Copy this line.</strong>
                <CopyLine text={overlay.launchOptions} />
                <p className="note">
                  <code>-usercon</code> turns on TF2's remote console, protected by a password made for you, so do not
                  share the line. <code>-condebug</code> writes the console log that match tracking reads. The radar
                  only sends <code>getpos</code>, <code>record</code> and <code>stop</code> for its own demo, empty{' '}
                  <code>alias</code> commands that make TF2 save the demo sooner, and a console filter that hides the{' '}
                  <code>getpos</code> replies.
                </p>
              </li>
              <li>
                In Steam: right-click <strong>Team Fortress 2</strong>, <strong>Properties</strong>,{' '}
                <strong>Launch Options</strong>. Keep anything already there.
              </li>
              <li>
                <strong>Restart TF2</strong> in Windowed (no border) mode. The live status takes this spot once the
                radar connects.
              </li>
            </ol>
            {linkBad ? (
              <p className="state bad">
                <TriangleAlert aria-hidden="true" />
                {status.link === 'unreachable'
                  ? 'TF2 is running without these options. Restart it after adding the line.'
                  : 'TF2 is running with an old password. Restart it with the line above.'}
              </p>
            ) : !on ? (
              <p className="state warn">
                {settings.enabled
                  ? 'The radar is off. Turn it on to connect.'
                  : 'The overlay is off. Turn it on to connect.'}
              </p>
            ) : setupDone ? (
              <p className="state ok">
                Applied.
                <span className="spacer" />
                <button type="button" className="link muted" onClick={() => setShowSetup(false)}>
                  Back to status
                </button>
              </p>
            ) : null}
          </div>
        ) : (
          <div>
            <dl className="rows">
              <Row label="Link" tone={linkBad ? 'bad' : linkIdle ? 'idle' : undefined}>
                {link}
              </Row>
              <Row label="Position" tone={status.positionHz === null ? 'idle' : undefined}>
                {status.positionHz === null ? (
                  connected ? (
                    'Starting'
                  ) : (
                    'Waiting for the link'
                  )
                ) : (
                  <>
                    Reading <code>getpos</code> over RCON <small>· {Math.round(status.positionHz)} per second</small>
                  </>
                )}
              </Row>
              <Row label="Demo" tone={status.demo ? undefined : 'idle'}>
                {status.demo ? (
                  <>
                    Following <code>{status.demo.file}</code>{' '}
                    <small>
                      ·{' '}
                      {status.demo.intervalS === null
                        ? 'measuring the write interval'
                        : `written every ${status.demo.intervalS.toFixed(1)} s`}
                      , filler {Math.round(status.demo.fillerKbps)} KB/s
                    </small>
                  </>
                ) : (
                  'Not recording'
                )}
              </Row>
              <Row label="Gargoyle" tone={status.gargoyle ? undefined : 'idle'}>
                {status.gargoyle ? (
                  <>
                    <strong className="ok">Found</strong> on {status.gargoyle.map ?? 'an unknown map'} at (
                    {Math.round(status.gargoyle.x)}, {Math.round(status.gargoyle.y)}, {Math.round(status.gargoyle.z)})
                    {status.gargoyle.leftS !== null && (
                      <small>
                        {' · '}
                        despawns in {clock(status.gargoyle.leftS)}
                      </small>
                    )}
                  </>
                ) : (
                  'None yet'
                )}
              </Row>
            </dl>
            {overlay.events.length === 0 ? (
              <p className="note events-empty">No events yet. Start TF2 to connect.</p>
            ) : (
              <ul className="events">
                {overlay.events.slice(0, 4).map((event) => (
                  <li key={`${event.at}:${event.text}`} className={event.tone}>
                    <time dateTime={new Date(event.at).toISOString()}>{time(event.at)}</time>
                    <span>{event.text}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </div>

      <Group title="Behaviour">
        <Field
          name="Demo delay"
          help="Longest wait before a new gargoyle shows. Lower is faster, but TF2 writes more to disk: at 5 s up to about 500 MB an hour on a quiet server. The file starts over at 100 MB."
        >
          <Slider
            label="Demo delay in seconds"
            value={settings.demoDelayS}
            min={2}
            max={20}
            step={1}
            format={(v) => `${v} s`}
            onInput={() => {}}
            onCommit={(demoDelayS) => {
              if (demoDelayS !== settings.demoDelayS) onChange({ demoDelayS });
            }}
          />
        </Field>
        <Field name="Units">
          <span className="choices" role="radiogroup" aria-label="Units">
            {RADAR_UNITS.map(({ value, label }) => (
              <label key={value}>
                <input
                  type="radio"
                  name="radarUnits"
                  value={value}
                  checked={settings.radarUnits === value}
                  onChange={() => onChange({ radarUnits: value })}
                />
                {label}
              </label>
            ))}
          </span>
        </Field>
        <Field name="Despawn countdown" help="How long until your gargoyle disappears, under the distance.">
          <Switch
            label="Despawn countdown"
            checked={settings.despawnTimer}
            onChange={(despawnTimer) => onChange({ despawnTimer })}
          />
        </Field>
        <Field
          name="RCON port"
          help={
            setupDone ? (
              <>
                Launch options applied.{' '}
                {!setup && (
                  <button type="button" className="link muted" onClick={() => setShowSetup(true)}>
                    Show launch options
                  </button>
                )}
              </>
            ) : (
              <span className="warn">Launch options not applied yet.</span>
            )
          }
        >
          <input
            type="number"
            aria-label="RCON port"
            min={1024}
            max={65535}
            value={portDraft ?? settings.port}
            onChange={(e) => setPortDraft(e.target.value)}
            onBlur={savePort}
            onKeyDown={(e) => {
              if (e.key === 'Enter') e.currentTarget.blur();
            }}
          />
        </Field>
        <Field
          name="Follow any gargoyle"
          help={
            <>
              Test mode for local servers: <code>sv_cheats 1</code>, then{' '}
              <code>ent_create tf_halloween_gift_pickup</code>.
            </>
          }
        >
          <input
            type="checkbox"
            aria-label="Follow any gargoyle"
            checked={settings.anyGargoyle}
            onChange={(e) => onChange({ anyGargoyle: e.target.checked })}
          />
        </Field>
      </Group>
    </>
  );
}
