import './Switch.css';

type SwitchProps = {
  checked: boolean;
  onChange: (checked: boolean) => void;
  /** What it turns on, e.g. "Merchant name" — the switch shows no text of its own. */
  ariaLabel: string;
  id?: string;
};

/** An on/off switch, for a setting that takes effect (or a section that opens) the moment it's
 * flipped. Use a checkbox instead for a choice inside a form that's saved later. */
export const Switch = ({ checked, onChange, ariaLabel, id }: SwitchProps) => (
  <button
    id={id}
    type="button"
    role="switch"
    aria-checked={checked}
    aria-label={ariaLabel}
    className={checked ? 'switch switch--on' : 'switch'}
    onClick={() => onChange(!checked)}
  >
    <span className="switch-thumb" aria-hidden="true" />
  </button>
);
