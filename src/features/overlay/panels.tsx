import { useState, type ReactNode } from 'react';

import type { CountdownFormat, OverlayWidget, OverlayWidgetPatch, RadarReading, RadarUnits } from '../../api.ts';
import { Field, Group, Slider } from '../../components/fields.tsx';

// The in-game panels' geometry in 1080p units (src-tauri/src/overlay/widgets: radar/panel.rs and countdown.rs); the previews draw them 1:1.
const PANEL_W = 232;
const RADAR_H = 100;
const COUNTDOWN_H = 56;
const RING = { cx: 56, cy: 50, rx: 40, ry: 18 };
const FLAT = RING.ry / RING.rx;
const CREAM = '#ebe3cb';
const TAN_DARK = '#756b5e';
const TAN_TEXT = '#9e9380';
const ORANGE = '#e58626';

export const clock = (s: number) => `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;

export const RADAR_UNITS: { value: RadarUnits; label: string }[] = [
  { value: 'metres', label: 'Metres' },
  { value: 'feet', label: 'Feet' },
  { value: 'hammer', label: 'Hammer units' }
];

// A hammer unit, TF2's own and what readings come in, is 0.75 inch (nav.rs METRES_PER_HU).
const METRES_PER_HU = 0.01905;
const hu = (metres: number) => metres / METRES_PER_HU;

// One of each unit: its length in hammer units, and its label (nav.rs Units::spec).
const UNIT: Record<RadarUnits, { size: number; sign: string }> = {
  hammer: { size: 1, sign: 'hu' },
  feet: { size: 16, sign: 'ft' },
  metres: { size: hu(1), sign: 'm' }
};

// A length in hammer units written as the panel writes it (nav.rs Units::amount): tenths under 10, then whole
// units; hammer units always whole.
export function amount(lengthHu: number, units: RadarUnits) {
  const v = lengthHu / UNIT[units].size;
  return `${units !== 'hammer' && v < 10 ? v.toFixed(1) : v.toFixed(0)} ${UNIT[units].sign}`;
}

// Shown until the radar has a live reading.
export const SAMPLE_READING: RadarReading = { angleDeg: 35, distHu: hu(23), heightHu: hu(4.6) };

// The panel's texts get what is right of the compass; a long one (hundreds of hammer units) shrinks to fit it,
// like in game. Widths are estimated from the font's average glyph width in em.
// The texts end at the column's right edge, so units and `left` stay put as the digits change (panel.rs TEXT_RIGHT).
const TEXT_RIGHT = 232 - 8;
const ROOM = TEXT_RIGHT - 112;
const fit = (text: string, px: number, em: number) => (text.length * px * em > ROOM ? ROOM : undefined);

// The in-game panel on a mid-tone like a map's wall, at the widget's opacity and scale, with the two sliders
// that set them under it. A slider mid-drag drives the preview before its value is saved.
export function Preview({
  widget,
  cap,
  onChange,
  children
}: {
  widget: OverlayWidget;
  cap?: ReactNode;
  onChange: (patch: OverlayWidgetPatch) => void;
  children: ReactNode;
}) {
  const [look, setLook] = useState<OverlayWidgetPatch>({});
  const opacity = look.opacity ?? widget.opacity;
  const scale = look.scale ?? widget.scale;
  return (
    <div>
      <div className="anchor">
        <div className="anchor-stage">
          <div style={{ opacity, transform: `scale(${scale})` }}>{children}</div>
        </div>
        {cap && <p className="cap">{cap}</p>}
      </div>
      <Group title="Look">
        <Field name="Opacity">
          <Slider
            label="Opacity"
            value={Math.round(widget.opacity * 100)}
            min={20}
            max={100}
            step={2}
            format={(v) => `${v}%`}
            onInput={(v) => setLook({ ...look, opacity: v / 100 })}
            onCommit={(v) => {
              setLook({});
              if (v / 100 !== widget.opacity) onChange({ opacity: v / 100 });
            }}
          />
        </Field>
        <Field name="Scale">
          <Slider
            label="Scale"
            value={widget.scale}
            min={0.5}
            max={2.5}
            step={0.05}
            format={(v) => `${v.toFixed(2)}x`}
            onInput={(v) => setLook({ ...look, scale: v })}
            onCommit={(v) => {
              setLook({});
              if (v !== widget.scale) onChange({ scale: v });
            }}
          />
        </Field>
      </Group>
    </div>
  );
}

// SVG replica of the radar: compass in perspective, the arrow on its floor, the height pin, the texts.
export function RadarPanel({
  reading,
  leftS,
  units
}: {
  reading: RadarReading;
  leftS: number | null;
  units: RadarUnits;
}) {
  const arrived = reading.distHu < hu(2.5);
  const level = Math.abs(reading.heightHu) < hu(1.5);
  const accent = arrived ? ORANGE : CREAM;
  const rad = (reading.angleDeg * Math.PI) / 180;
  const [sin, cos] = [Math.sin(rad), Math.cos(rad)];
  // The arrow lies on the ground plane: forward is up the screen, foreshortened by the ring's flatness.
  const ground = (side: number, fwd: number) => ({ x: RING.cx + side, y: RING.cy - fwd * FLAT });
  const tip = arrived ? { x: RING.cx, y: RING.cy } : ground(sin * 0.85 * RING.rx, cos * 0.85 * RING.rx);
  const base = ground(-sin * 0.35 * RING.rx, -cos * 0.35 * RING.rx);
  const wing = { x: cos * 0.42 * RING.rx, y: sin * 0.42 * RING.rx * Math.sqrt(FLAT) };
  const pinLen = level ? 0 : Math.max(-24, Math.min(24, reading.heightHu * METRES_PER_HU * 4));
  const pin = (
    <>
      <path
        d={`M${tip.x} ${tip.y} L${tip.x} ${tip.y - pinLen}`}
        stroke={accent}
        strokeWidth="2"
        strokeLinecap="round"
      />
      <circle cx={tip.x} cy={tip.y - pinLen} r="2.5" fill={accent} stroke="#1a1816" />
    </>
  );

  const distance = arrived ? 'HERE' : amount(reading.distHu, units);
  const height = arrived
    ? level
      ? 'level'
      : reading.heightHu > 0
        ? 'look up'
        : 'look down'
    : level
      ? 'level'
      : `${amount(Math.abs(reading.heightHu), units)} ${reading.heightHu > 0 ? 'above' : 'below'}`;
  const heightColour = arrived ? ORANGE : level || reading.heightHu < 0 ? TAN_TEXT : CREAM;

  return (
    <svg
      className="panel"
      viewBox={`0 0 ${PANEL_W + 2} ${RADAR_H + 2}`}
      width={PANEL_W}
      height={RADAR_H}
      aria-hidden="true"
    >
      <rect x="1" y="1" width={PANEL_W} height={RADAR_H} rx="4" fill="#000" opacity="0.6" />
      <rect width={PANEL_W} height={RADAR_H} rx="4" fill="#2e2b2a" />
      {/* Below you: the pin hangs under the floor, so the ring is drawn over it. */}
      {pinLen < 0 && pin}
      <path d="M16 50 A40 18 0 0 1 96 50" fill="none" stroke={TAN_DARK} strokeWidth="2.5" strokeLinecap="round" />
      <path d="M56 32 L56 26" stroke={TAN_DARK} strokeWidth="2" strokeLinecap="round" />
      <path d="M16 50 A40 18 0 0 0 96 50" fill="none" stroke={accent} strokeWidth="2.5" strokeLinecap="round" />
      {!arrived && (
        <path
          d={`M${tip.x} ${tip.y} L${base.x + wing.x} ${base.y + wing.y} L${RING.cx} ${RING.cy} L${base.x - wing.x} ${base.y - wing.y} Z`}
          fill={CREAM}
        />
      )}
      {pinLen >= 0 && pin}
      <text
        className="big"
        x={TEXT_RIGHT}
        textAnchor="end"
        y="44"
        fill={accent}
        textLength={fit(distance, 30, 0.68)}
        lengthAdjust="spacingAndGlyphs"
      >
        {distance}
      </text>
      <text
        className="small"
        x={TEXT_RIGHT}
        textAnchor="end"
        y="70"
        fill={heightColour}
        textLength={fit(height, 15, 0.5)}
        lengthAdjust="spacingAndGlyphs"
      >
        {height}
      </text>
      {leftS !== null && (
        <text className="small" x={TEXT_RIGHT} textAnchor="end" y="90" fill={leftS <= 30 ? ORANGE : TAN_TEXT}>
          {`${clock(leftS)} left`}
        </text>
      )}
    </svg>
  );
}

export type Countdown = { big: string; small: string; urgency: 'calm' | 'lastDay' | 'ended' };

const DAY = 86_400;
const HOUR = 3_600;
const two = (n: number) => String(n).padStart(2, '0');

// The end from TF2's "YYYY-MM-DD": the start of that day in UTC, since Valve gives no time zone.
export function endDate(date: string | null) {
  const end = date ? new Date(`${date}T00:00:00Z`) : null;
  return end && !Number.isNaN(end.getTime()) ? end : null;
}

const endShort = (end: Date) =>
  end.toLocaleString('en-GB', { weekday: 'short', day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });

export const COUNTDOWN_FORMATS: { value: CountdownFormat; label: string }[] = [
  { value: 'adaptive', label: 'Adaptive' },
  { value: 'days', label: 'Days' },
  { value: 'detailed', label: 'Detailed' },
  { value: 'clock', label: 'Clock' }
];

// The big line leftS seconds before the end, in each format; the same wording as countdown.rs.
function big(leftS: number, format: CountdownFormat) {
  const days = Math.floor(leftS / DAY);
  const hours = Math.floor((leftS % DAY) / HOUR);
  const minutes = Math.floor((leftS % HOUR) / 60);
  const seconds = leftS % 60;
  switch (format) {
    case 'adaptive':
      if (leftS >= 3 * DAY) return `${days} days`;
      if (leftS >= DAY) return `${days} d ${hours} h`;
      if (leftS >= HOUR) return `${hours} h ${two(minutes)} m`;
      return `0:${two(minutes)}:${two(seconds)}`;
    case 'days':
      return days === 0 ? 'Today' : days === 1 ? 'In 1 day' : `In ${days} days`;
    case 'detailed':
      return days === 0 ? `${hours} h ${two(minutes)} m` : `${days} d ${hours} h ${two(minutes)} m`;
    case 'clock':
      return `${two(days)}:${two(hours)}:${two(minutes)}:${two(seconds)}`;
  }
}

// What the widget shows leftS seconds before the end. The last day is orange whatever the format; from a day
// after the end it is null, when the widget hides.
export function countdownText(leftS: number, end: Date, format: CountdownFormat): Countdown | null {
  if (leftS <= -DAY) return null;
  if (leftS <= 0) {
    const date = end.toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
    return { big: 'Ended', small: `Contracts ended ${date}`, urgency: 'ended' };
  }
  return {
    big: big(leftS, format),
    small: `Contracts end ${endShort(end)}`,
    urgency: leftS < DAY ? 'lastDay' : 'calm'
  };
}

// What edit mode shows while TF2's files give no end date: 21 days ahead of a sample end.
export const SAMPLE_LEFT_S = 21 * DAY;
export const SAMPLE_END = new Date(new Date().getFullYear(), 10, 8, 1);

// SVG replica of the countdown: the time left large, the end date under it.
export function CountdownPanel({ countdown }: { countdown: Countdown }) {
  const [big, small] =
    countdown.urgency === 'calm'
      ? [CREAM, TAN_TEXT]
      : countdown.urgency === 'lastDay'
        ? [ORANGE, ORANGE]
        : [TAN_TEXT, TAN_TEXT];
  return (
    <svg
      className="panel"
      viewBox={`0 0 ${PANEL_W + 2} ${COUNTDOWN_H + 2}`}
      width={PANEL_W}
      height={COUNTDOWN_H}
      aria-hidden="true"
    >
      <rect x="1" y="1" width={PANEL_W} height={COUNTDOWN_H} rx="4" fill="#000" opacity="0.6" />
      <rect width={PANEL_W} height={COUNTDOWN_H} rx="4" fill="#2e2b2a" />
      <text className="mid" x="12" y="30" fill={big}>
        {countdown.big}
      </text>
      <text className="tiny" x="12" y="48" fill={small}>
        {countdown.small}
      </text>
    </svg>
  );
}
