import { useState, type ReactNode } from 'react';

// One level of grouping: a heading on a rule line, then rows on the page background.
export function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="group">
      <div className="group-head">
        <h3>{title}</h3>
      </div>
      {children}
    </div>
  );
}

export function Field({
  name,
  help,
  wide,
  children
}: {
  name: string;
  help?: ReactNode;
  wide?: boolean;
  children?: ReactNode;
}) {
  return (
    <div className={wide ? 'field wide' : 'field'}>
      <span className="name">{name}</span>
      {help && <span className="help">{help}</span>}
      {children && <span className="control">{children}</span>}
    </div>
  );
}

export function Switch({
  label,
  checked,
  disabled,
  big,
  onChange
}: {
  label: string;
  checked: boolean;
  disabled?: boolean;
  big?: boolean;
  onChange: (next: boolean) => void;
}) {
  return (
    <button
      type="button"
      className={big ? 'toggle big' : 'toggle'}
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    />
  );
}

export function Slider({
  label,
  value,
  min,
  max,
  step,
  format,
  onInput,
  onCommit
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  format: (value: number) => string;
  onInput: (value: number) => void;
  onCommit: (value: number) => void;
}) {
  // The value being dragged. It feeds the preview as it moves and is sent once the drag or key press ends.
  const [draft, setDraft] = useState<number | null>(null);
  const shown = draft ?? value;
  const commit = () => {
    if (draft === null) return;
    setDraft(null);
    onCommit(draft);
  };
  return (
    <>
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={shown}
        onChange={(e) => {
          setDraft(e.target.valueAsNumber);
          onInput(e.target.valueAsNumber);
        }}
        onPointerUp={commit}
        onKeyUp={commit}
        onBlur={commit}
      />
      <span className="val">{format(shown)}</span>
    </>
  );
}
