export type LayoutId = "studio" | "focus" | "mission" | "conversation";
export type NodeStyle = "card" | "compact" | "terminal";
export type EdgeStyle = "bezier" | "orthogonal" | "flow";
export type Surface = "obsidiana" | "grafite" | "mineral";
export type StateScheme = "brand" | "semantic";
export type CanvasBackground = "dots" | "grid" | "diagonal";

export type Choice = {
  layout: LayoutId;
  node: NodeStyle;
  edge: EdgeStyle;
  surface: Surface;
  states: StateScheme;
  background: CanvasBackground;
};

export type AxisKey = Exclude<keyof Choice, "layout">;

type Option<T extends string> = { value: T; label: string; hint: string };

export const LAYOUTS: (Option<LayoutId> & { letter: string })[] = [
  { value: "studio", letter: "A", label: "Studio", hint: "Tradução direta da referência: árvore, canvas, inspector, Run Card e console." },
  { value: "focus", letter: "B", label: "Foco", hint: "Canvas na janela inteira com painéis flutuantes, minimapa e dock." },
  { value: "mission", letter: "C", label: "Mission Control", hint: "Canvas em cima e timeline das Executions por Agent Slot embaixo." },
  { value: "conversation", letter: "D", label: "Conversa", hint: "Canvas e painel com sessão do agente, perguntas e aprovações." },
];

export const AXES: { key: AxisKey; label: string; options: Option<string>[] }[] = [
  {
    key: "node",
    label: "Nó",
    options: [
      { value: "card", label: "Cartão", hint: "Cartão com portas; cada porta é um artefato." },
      { value: "compact", label: "Compacto", hint: "Pílula com anel de status e avatar do agente." },
      { value: "terminal", label: "Terminal", hint: "Final do log da Execution em mono." },
    ],
  },
  {
    key: "edge",
    label: "Aresta",
    options: [
      { value: "bezier", label: "Bezier", hint: "Curvas como na referência." },
      { value: "orthogonal", label: "Ortogonal", hint: "Ângulos retos, leitura de diagrama." },
      { value: "flow", label: "Fluxo", hint: "Lava→Brasa correndo nas arestas ativas." },
    ],
  },
  {
    key: "surface",
    label: "Superfície",
    options: [
      { value: "obsidiana", label: "Obsidiana", hint: "Escura, base 60% da marca." },
      { value: "grafite", label: "Grafite", hint: "Painéis elevados, mais contraste entre camadas." },
      { value: "mineral", label: "Mineral", hint: "Clara; Lava só em fundos." },
    ],
  },
  {
    key: "states",
    label: "Estados",
    options: [
      { value: "brand", label: "Só marca", hint: "Lava, Brasa, Mineral e hachura diagonal." },
      { value: "semantic", label: "Marca + semânticas", hint: "Acrescenta verde e carmim." },
    ],
  },
  {
    key: "background",
    label: "Fundo",
    options: [
      { value: "dots", label: "Pontos", hint: "Como a referência." },
      { value: "grid", label: "Grade", hint: "Grade de 24px do notebook." },
      { value: "diagonal", label: "Diagonais", hint: "O gesto do símbolo, sutil." },
    ],
  },
];

export const PRESETS: Record<LayoutId, Choice> = {
  studio: { layout: "studio", node: "card", edge: "bezier", surface: "obsidiana", states: "brand", background: "dots" },
  focus: { layout: "focus", node: "compact", edge: "flow", surface: "obsidiana", states: "brand", background: "diagonal" },
  mission: { layout: "mission", node: "card", edge: "orthogonal", surface: "grafite", states: "semantic", background: "grid" },
  conversation: { layout: "conversation", node: "terminal", edge: "bezier", surface: "obsidiana", states: "brand", background: "dots" },
};

export function layoutOf(id: LayoutId) {
  return LAYOUTS.find((layout) => layout.value === id) ?? LAYOUTS[0];
}

export function describeChoice(choice: Choice) {
  const axes = AXES.map((axis) => {
    const option = axis.options.find((item) => item.value === choice[axis.key]);
    return `${axis.label}: ${option?.label ?? choice[axis.key]}`;
  });
  const layout = layoutOf(choice.layout);
  return [`Layout: ${layout.letter} · ${layout.label}`, ...axes];
}
