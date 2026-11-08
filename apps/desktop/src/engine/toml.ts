export type TomlValue = string | number | boolean | TomlValue[] | TomlTable;
export type TomlTable = { [key: string]: TomlValue };

const BARE = /[A-Za-z0-9_-]/;
const ESCAPES: Record<string, string> = { n: "\n", t: "\t", r: "\r", '"': '"', "\\": "\\", b: "\b", f: "\f" };

export function parseToml(text: string): TomlTable {
  return new Parser(text).document();
}

class Parser {
  private at = 0;
  private text: string;

  constructor(text: string) {
    this.text = text.replace(/\r\n/g, "\n");
  }

  document(): TomlTable {
    const root: TomlTable = {};
    let current = root;
    for (this.blank(); this.at < this.text.length; this.blank()) {
      if (this.peek() === "[") {
        const many = this.text.startsWith("[[", this.at);
        this.at += many ? 2 : 1;
        const path = this.keyPath();
        this.expect(many ? "]]" : "]");
        current = many ? appendTable(root, path) : tableAt(root, path);
      } else {
        const path = this.keyPath();
        this.expect("=");
        assign(current, path, this.value());
      }
      this.endOfLine();
    }
    return root;
  }

  private value(): TomlValue {
    this.spaces();
    if (this.text.startsWith('"""', this.at)) return this.multiline('"""', true);
    if (this.text.startsWith("'''", this.at)) return this.multiline("'''", false);
    const next = this.peek();
    if (next === '"') return this.basic();
    if (next === "'") return this.literal();
    if (next === "[") return this.array();
    if (next === "{") return this.inline();
    const word = /^[^\s,\]}#]+/.exec(this.text.slice(this.at))?.[0] ?? "";
    if (!word) throw new Error(`TOML value expected at ${this.at}`);
    this.at += word.length;
    if (word === "true" || word === "false") return word === "true";
    const number = Number(word.replace(/_/g, ""));
    return Number.isNaN(number) ? word : number;
  }

  private array(): TomlValue[] {
    const items: TomlValue[] = [];
    this.at += 1;
    for (this.blank(); this.peek() !== "]"; this.blank()) {
      items.push(this.value());
      this.blank();
      if (this.peek() === ",") this.at += 1;
      if (this.at >= this.text.length) throw new Error("unterminated TOML array");
    }
    this.at += 1;
    return items;
  }

  private inline(): TomlTable {
    const table: TomlTable = {};
    this.at += 1;
    for (this.spaces(); this.peek() !== "}"; this.spaces()) {
      const path = this.keyPath();
      this.expect("=");
      assign(table, path, this.value());
      this.spaces();
      if (this.peek() === ",") this.at += 1;
      if (this.at >= this.text.length) throw new Error("unterminated TOML inline table");
    }
    this.at += 1;
    return table;
  }

  private keyPath(): string[] {
    const path: string[] = [];
    do {
      this.spaces();
      const next = this.peek();
      if (next === '"') path.push(this.basic());
      else if (next === "'") path.push(this.literal());
      else path.push(this.bare());
      this.spaces();
    } while (this.peek() === "." && ++this.at);
    return path;
  }

  private bare(): string {
    const start = this.at;
    while (this.at < this.text.length && BARE.test(this.text[this.at])) this.at += 1;
    if (start === this.at) throw new Error(`TOML key expected at ${start}`);
    return this.text.slice(start, this.at);
  }

  private basic(): string {
    let out = "";
    for (this.at += 1; this.peek() !== '"'; this.at += 1) {
      if (this.at >= this.text.length || this.peek() === "\n") throw new Error("unterminated TOML string");
      if (this.peek() === "\\") out += ESCAPES[this.text[++this.at]] ?? this.text[this.at];
      else out += this.peek();
    }
    this.at += 1;
    return out;
  }

  private literal(): string {
    const end = this.text.indexOf("'", this.at + 1);
    if (end < 0) throw new Error("unterminated TOML string");
    const out = this.text.slice(this.at + 1, end);
    this.at = end + 1;
    return out;
  }

  private multiline(fence: string, escapes: boolean): string {
    const end = this.text.indexOf(fence, this.at + 3);
    if (end < 0) throw new Error("unterminated TOML string");
    const raw = this.text.slice(this.at + 3, end).replace(/^\n/, "");
    this.at = end + 3;
    return escapes ? raw.replace(/\\\n\s*/g, "").replace(/\\(.)/g, (_, char: string) => ESCAPES[char] ?? char) : raw;
  }

  private expect(token: string) {
    this.spaces();
    if (!this.text.startsWith(token, this.at)) throw new Error(`TOML expected "${token}" at ${this.at}`);
    this.at += token.length;
  }

  private endOfLine() {
    this.spaces();
    if (this.peek() === "#") while (this.at < this.text.length && this.peek() !== "\n") this.at += 1;
  }

  private spaces() {
    while (this.peek() === " " || this.peek() === "\t") this.at += 1;
  }

  private blank() {
    while (this.at < this.text.length) {
      const next = this.peek();
      if (next === "#") while (this.at < this.text.length && this.peek() !== "\n") this.at += 1;
      else if (/\s/.test(next)) this.at += 1;
      else return;
    }
  }

  private peek() {
    return this.text[this.at];
  }
}

function tableAt(root: TomlTable, path: string[]): TomlTable {
  let table = root;
  for (const key of path) {
    const next = table[key];
    if (Array.isArray(next)) table = next[next.length - 1] as TomlTable;
    else if (next && typeof next === "object") table = next;
    else table = table[key] = {};
  }
  return table;
}

function appendTable(root: TomlTable, path: string[]): TomlTable {
  const parent = tableAt(root, path.slice(0, -1));
  const key = path[path.length - 1];
  const list = Array.isArray(parent[key]) ? (parent[key] as TomlValue[]) : (parent[key] = []);
  const table: TomlTable = {};
  list.push(table);
  return table;
}

function assign(table: TomlTable, path: string[], value: TomlValue) {
  tableAt(table, path.slice(0, -1))[path[path.length - 1]] = value;
}
