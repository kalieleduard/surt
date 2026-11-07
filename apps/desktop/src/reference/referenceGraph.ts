export type PortKind = "idle" | "linked" | "hot";
export type PortSide = "in" | "out";

export type NodeRow = {
  left?: string;
  right?: string;
  inPort?: PortKind;
  outPort?: PortKind;
  lead?: "open" | "closed" | "leaf";
  strong?: boolean;
};

export type GraphNode = {
  title: string;
  x: number;
  y: number;
  width: number;
  rows: NodeRow[];
  marked?: boolean;
};

export type Point = { x: number; y: number };

export type GraphEdge = { id: string; from: Point; to: Point; hot?: boolean };

const FIRST_ROW_CENTER = 31;
const ROW_HEIGHT = 16;
const PORT_INSET = 7;
const NODE_WIDTH = 160;

export const CANVAS_EDGE_X = 767;

export const INSPECTOR_PORTS = { className: 200.5, id: 241.5, bottom: 508.5 };

export const INSPECTOR_IDLE_PORTS = [344.5, 385.5, 426.5, 467.5, 549.5, 590.5];

export const NODES = {
  route012: {
    title: "Route 0.1.2",
    x: 268,
    y: 156,
    width: 166,
    rows: [{ left: "args", inPort: "idle", right: "path", outPort: "idle" }],
  },
  hidden: {
    title: "SetPosition",
    x: 175,
    y: 508,
    width: NODE_WIDTH,
    rows: [
      { left: "args", inPort: "idle", right: "path", outPort: "idle" },
      { left: "id", inPort: "idle", right: "x", outPort: "idle" },
      { left: "title", inPort: "idle", right: "y", outPort: "idle" },
    ],
  },
  button: {
    title: "Button",
    x: 323,
    y: 60,
    width: NODE_WIDTH,
    marked: true,
    rows: [
      { left: "args", inPort: "idle", right: "OnClick", outPort: "hot" },
      { right: "OnDoubleClick", outPort: "hot" },
    ],
  },
  addClass: {
    title: "AddClass",
    x: 504,
    y: 60,
    width: NODE_WIDTH,
    rows: [{ left: "args", inPort: "hot", right: "animateIt", outPort: "linked" }],
  },
  pageTransition: {
    title: "PageTransition",
    x: 504,
    y: 110,
    width: NODE_WIDTH,
    rows: [{ left: "args", inPort: "hot", right: "path", outPort: "linked" }],
  },
  route011: {
    title: "Route 0.1.1",
    x: 504,
    y: 161,
    width: NODE_WIDTH,
    rows: [{ left: "args", inPort: "linked", right: "path", outPort: "idle" }],
  },
  data: {
    title: "Data",
    x: 323,
    y: 258,
    width: NODE_WIDTH,
    rows: [
      { left: "Glossary", right: "{}", lead: "open", strong: true, outPort: "idle" },
      { left: "Title", right: "example glo…", lead: "leaf", outPort: "idle" },
      { left: "TitleDiv", right: "{}", lead: "closed", outPort: "linked" },
      { left: "SortAs", right: "[]", lead: "closed", outPort: "idle" },
      { left: "GlossTerm", right: "10", lead: "leaf", outPort: "idle" },
      { left: "Acronym", right: "{}", lead: "closed", outPort: "linked" },
    ],
  },
  headerText: {
    title: "headerText",
    x: 513,
    y: 260,
    width: NODE_WIDTH,
    rows: [
      { left: "Title", inPort: "linked", right: "TitleName", outPort: "idle" },
      { left: "ID", inPort: "linked", right: "ID", outPort: "linked" },
    ],
  },
  priceValue: {
    title: "priceValue",
    x: 532,
    y: 359,
    width: NODE_WIDTH,
    rows: [
      { left: "Title", inPort: "linked", right: "Title", outPort: "linked" },
      { left: "ID", inPort: "linked", right: "ID", outPort: "idle" },
    ],
  },
  setMargin: {
    title: "SetMargin",
    x: 582,
    y: 448,
    width: NODE_WIDTH,
    rows: [
      { left: "args", inPort: "linked", right: "path", outPort: "idle" },
      { left: "id", inPort: "idle", right: "x", outPort: "linked" },
      { left: "title", inPort: "idle", right: "y", outPort: "idle" },
    ],
  },
} satisfies Record<string, GraphNode>;

export const CODE_NODE = { x: 361, y: 413, width: 191 };

export const CODE_PORTS = {
  input: { x: CODE_NODE.x - 12, y: CODE_NODE.y + 48 },
  stub: { x: CODE_NODE.x - 5, y: CODE_NODE.y + 48 },
  output: { x: CODE_NODE.x + CODE_NODE.width + 6, y: CODE_NODE.y + 60 },
};

export function portPoint(node: GraphNode, row: number, side: PortSide): Point {
  const x = side === "in" ? node.x + PORT_INSET : node.x + node.width - PORT_INSET;
  return { x, y: node.y + FIRST_ROW_CENTER + ROW_HEIGHT * row };
}

export function curvePath(from: Point, to: Point) {
  const bend = (to.x - from.x) / 2;
  return `M${from.x} ${from.y}C${from.x + bend} ${from.y} ${to.x - bend} ${to.y} ${to.x} ${to.y}`;
}

function toInspector(y: number): Point {
  return { x: CANVAS_EDGE_X, y };
}

export const HOT_SPAN = {
  from: portPoint(NODES.button, 0, "out").x,
  to: portPoint(NODES.addClass, 0, "in").x,
};

export const EDGES: GraphEdge[] = [
  { id: "click", from: portPoint(NODES.button, 0, "out"), to: portPoint(NODES.addClass, 0, "in"), hot: true },
  { id: "double", from: portPoint(NODES.button, 1, "out"), to: portPoint(NODES.pageTransition, 0, "in"), hot: true },
  { id: "route", from: portPoint(NODES.pageTransition, 0, "out"), to: portPoint(NODES.route011, 0, "in") },
  { id: "class", from: portPoint(NODES.addClass, 0, "out"), to: toInspector(INSPECTOR_PORTS.className) },
  { id: "title", from: portPoint(NODES.data, 2, "out"), to: portPoint(NODES.headerText, 0, "in") },
  { id: "title-id", from: portPoint(NODES.data, 2, "out"), to: portPoint(NODES.headerText, 1, "in") },
  { id: "price", from: portPoint(NODES.data, 5, "out"), to: portPoint(NODES.priceValue, 0, "in") },
  { id: "price-id", from: portPoint(NODES.data, 5, "out"), to: portPoint(NODES.priceValue, 1, "in") },
  { id: "my-id", from: portPoint(NODES.headerText, 1, "out"), to: toInspector(INSPECTOR_PORTS.id) },
  { id: "header", from: portPoint(NODES.priceValue, 0, "out"), to: CODE_PORTS.input },
  { id: "margin", from: CODE_PORTS.output, to: portPoint(NODES.setMargin, 0, "in") },
  { id: "bottom", from: portPoint(NODES.setMargin, 1, "out"), to: toInspector(INSPECTOR_PORTS.bottom) },
];
