import { useCallback, useRef, useState } from "react";
import { Check, Palette } from "lucide-react";
import { useDismiss } from "../ui/useDismiss";
import { THEMES, useTheme } from "./ThemeContext";

export function ThemePicker() {
  const { theme, setTheme } = useTheme();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLSpanElement>(null);
  const close = useCallback(() => setOpen(false), []);
  useDismiss(ref, open, close);

  return (
    <span className="theme-picker" ref={ref}>
      <button className="icon-btn" aria-pressed={open} aria-haspopup="menu" onClick={() => setOpen(!open)} title="Theme">
        <Palette size={15} />
      </button>
      {open ? (
        <div className="theme-picker__menu" role="menu">
          <span className="caption theme-picker__title">Theme</span>
          {THEMES.map((option) => (
            <button
              key={option.value}
              role="menuitemradio"
              aria-checked={option.value === theme}
              className={`theme-picker__item${option.value === theme ? " is-active" : ""}`}
              onClick={() => {
                setTheme(option.value);
                close();
              }}
            >
              <span className="theme-picker__swatch" style={{ background: option.swatch }} />
              <span className="theme-picker__text">
                <span>{option.label}</span>
                <small>{option.hint}</small>
              </span>
              {option.value === theme ? <Check size={14} /> : null}
            </button>
          ))}
        </div>
      ) : null}
    </span>
  );
}
