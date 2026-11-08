import { memo, useMemo, useRef, type KeyboardEvent } from "react";
import { tokenize, type Language } from "./languages";
import "./code.css";

const INDENT = "  ";

export const Highlighted = memo(function Highlighted({ text, language }: { text: string; language: Language }) {
  const tokens = useMemo(() => tokenize(text, language), [text, language]);
  return (
    <>
      {tokens.map((token, index) =>
        token.kind ? (
          <span key={index} className={`tok-${token.kind}`}>
            {token.text}
          </span>
        ) : (
          token.text
        ),
      )}
    </>
  );
});

export function CodeBlock({ text, language, className = "" }: { text: string; language: Language; className?: string }) {
  return (
    <pre className={`code-block mono ${className}`}>
      <Highlighted text={text} language={language} />
    </pre>
  );
}

type EditorProps = {
  value: string;
  language: Language;
  onChange: (text: string) => void;
  onSave?: () => void;
  wrap?: boolean;
  gutter?: boolean;
  autoFocus?: boolean;
  className?: string;
};

export function CodeEditor({ value, language, onChange, onSave, wrap = false, gutter = !wrap, autoFocus, className = "" }: EditorProps) {
  const area = useRef<HTMLTextAreaElement>(null);
  const lines = useMemo(() => value.split("\n").length, [value]);

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      onSave?.();
      return;
    }
    if (event.key !== "Tab") return;
    event.preventDefault();
    const target = event.currentTarget;
    const { selectionStart, selectionEnd } = target;
    if (event.shiftKey) return outdent(target, value, onChange);
    onChange(value.slice(0, selectionStart) + INDENT + value.slice(selectionEnd));
    requestAnimationFrame(() => target.setSelectionRange(selectionStart + INDENT.length, selectionStart + INDENT.length));
  };

  return (
    <div className={`code-editor${wrap ? " is-wrapped" : ""} ${className}`} onClick={() => area.current?.focus()}>
      {gutter ? (
        <div className="code-editor__gutter mono" aria-hidden>
          {Array.from({ length: lines }, (_, index) => (
            <span key={index}>{index + 1}</span>
          ))}
        </div>
      ) : null}
      <div className="code-editor__sheet">
        <pre className="code-editor__paint mono" aria-hidden>
          <Highlighted text={value} language={language} />
          {"\n"}
        </pre>
        <textarea
          ref={area}
          className="code-editor__input mono"
          value={value}
          spellCheck={false}
          autoFocus={autoFocus}
          wrap={wrap ? "soft" : "off"}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={onKeyDown}
        />
      </div>
    </div>
  );
}

function outdent(target: HTMLTextAreaElement, value: string, onChange: (text: string) => void) {
  const { selectionStart } = target;
  const lineStart = value.lastIndexOf("\n", selectionStart - 1) + 1;
  if (!value.startsWith(INDENT, lineStart)) return;
  onChange(value.slice(0, lineStart) + value.slice(lineStart + INDENT.length));
  const caret = Math.max(lineStart, selectionStart - INDENT.length);
  requestAnimationFrame(() => target.setSelectionRange(caret, caret));
}
