import { useMemo, useState, type KeyboardEvent, type ReactNode } from "react";
import { Check, Plus } from "lucide-react";

export type PickerItem = {
  key: string;
  icon: ReactNode;
  title: string;
  details: string[];
  checked?: boolean;
  search?: string;
  onPick: () => void;
};

export type PickerSection = { title: string; items: PickerItem[] };

type Create = { label: (query: string) => string | null; onCreate: (query: string) => void };

type Row = { kind: "create"; label: string } | { kind: "item"; item: PickerItem };

export function Picker({
  placeholder,
  sections,
  create,
  busy,
  error,
  footer,
}: {
  placeholder: string;
  sections: PickerSection[];
  create?: Create;
  busy?: boolean;
  error?: string | null;
  footer?: ReactNode;
}) {
  const [query, setQuery] = useState("");
  const [cursor, setCursor] = useState(0);
  const needle = query.trim().toLowerCase();
  const shown = useMemo(
    () =>
      sections
        .map((section) => ({
          ...section,
          items: section.items.filter((item) => (item.search ?? `${item.title} ${item.details.join(" ")}`).toLowerCase().includes(needle)),
        }))
        .filter((section) => section.items.length),
    [sections, needle],
  );
  const createLabel = create?.label(query.trim()) ?? null;
  const rows: Row[] = [
    ...(createLabel ? [{ kind: "create" as const, label: createLabel }] : []),
    ...shown.flatMap((section) => section.items.map((item) => ({ kind: "item" as const, item }))),
  ];
  const active = Math.min(cursor, Math.max(rows.length - 1, 0));

  const choose = (row: Row | undefined) => {
    if (!row || busy) return;
    if (row.kind === "create") create?.onCreate(query.trim());
    else row.item.onPick();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setCursor((active + 1) % Math.max(rows.length, 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setCursor((active - 1 + rows.length) % Math.max(rows.length, 1));
    } else if (event.key === "Enter") {
      event.preventDefault();
      choose(rows[active]);
    }
  };

  let index = createLabel ? 1 : 0;
  return (
    <div className="picker" role="dialog">
      <input
        className="picker__input"
        autoFocus
        placeholder={placeholder}
        value={query}
        onChange={(event) => {
          setQuery(event.target.value);
          setCursor(0);
        }}
        onKeyDown={onKeyDown}
      />
      <div className="picker__list" role="listbox">
        {createLabel ? (
          <button className={`picker__create${active === 0 ? " is-active" : ""}`} onClick={() => choose(rows[0])} onMouseEnter={() => setCursor(0)} disabled={busy}>
            <Plus size={14} /> <span>{createLabel}</span>
          </button>
        ) : null}
        {shown.map((section) => (
          <div key={section.title} className="picker__section">
            <span className="picker__section-title">{section.title}</span>
            {section.items.map((item) => {
              const position = index++;
              return (
                <button
                  key={item.key}
                  role="option"
                  aria-selected={position === active}
                  className={`picker__item${position === active ? " is-active" : ""}`}
                  onClick={() => choose({ kind: "item", item })}
                  onMouseEnter={() => setCursor(position)}
                  disabled={busy}
                >
                  <span className="picker__icon">{item.checked ? <Check size={14} className="picker__check" /> : item.icon}</span>
                  <span className="picker__text">
                    <span className="picker__title">{item.title}</span>
                    {item.details.length ? (
                      <span className="picker__details mono">
                        {item.details.map((detail, at) => (
                          <span key={at}>{detail}</span>
                        ))}
                      </span>
                    ) : null}
                  </span>
                </button>
              );
            })}
          </div>
        ))}
        {!rows.length ? <p className="picker__empty">Nothing matches</p> : null}
      </div>
      {error ? <p className="picker__error">{error}</p> : null}
      {footer ? <div className="picker__footer">{footer}</div> : null}
    </div>
  );
}
