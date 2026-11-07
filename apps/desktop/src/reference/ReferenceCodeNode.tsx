import { CODE_NODE } from "./referenceGraph";

type TokenKind = "key" | "param" | "tag" | "attr" | "text" | "var";
type Token = [TokenKind, string];

const HEADER_SOURCE: Token[][] = [
  [["key", "export default function"]],
  [["text", "Header("], ["param", "title"], ["text", ", "], ["param", "id"], ["text", ") {"]],
  [["text", "  "], ["key", "return"], ["text", " ("]],
  [["tag", "    <div"], ["attr", " className="], ["text", "\"App\""], ["tag", ">"]],
  [["tag", "      <h1>"]],
  [["var", "        $title $so"]],
  [["tag", "      </h1>"]],
  [["tag", "      <h2>"]],
  [["var", "        $id"]],
  [["tag", "      </h2>"]],
  [["tag", "    </div>"]],
  [["text", "  );"]],
  [["text", "}"]],
];

const CARET_LINE = 5;

export function ReferenceCodeNode() {
  return (
    <div className="ref-code" style={{ left: CODE_NODE.x, top: CODE_NODE.y, width: CODE_NODE.width }}>
      <div className="ref-code-head">
        <span>Header</span>
        <span className="ref-code-done">Done</span>
      </div>
      <pre className="ref-code-body">
        {HEADER_SOURCE.map((line, index) => (
          <span key={index} className="ref-code-line">
            {line.map(([kind, text]) => (
              <span key={`${kind}-${text}`} className={`ref-tok-${kind}`}>
                {text}
              </span>
            ))}
            {index === CARET_LINE && <span className="ref-code-caret" />}
          </span>
        ))}
      </pre>
    </div>
  );
}
