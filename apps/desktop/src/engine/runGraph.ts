import type { GraphNode, RunGraph, StepData } from "../data/types";
import { runEdges } from "./runEdges";
import { runNodes, type RunSource } from "./runNodes";
import { foldStepRuns } from "./stepRuns";
import { definedSteps } from "./definedSteps";
import { parseToml } from "./toml";
import type { ProjectView, RunDetail, SubjectView } from "./types";

const ATTENTION = ["waiting", "blocked", "running", "interrupted"];

export function graphId(subject: string, item: string | null) {
  return `${subject}/${item ?? ""}`;
}

export function buildRunGraph(project: ProjectView, subject: SubjectView, item: string | null, detail: RunDetail): RunGraph {
  const root = item === null;
  const source: RunSource = {
    steps: new Map(detail.definition.steps.map((step) => [step.key, step])),
    runs: foldStepRuns(detail.events),
    driven: subject.driven,
    items: root ? subject.items : [],
    itemStep: root ? subject.itemStep : null,
    gates: root ? subject.gates : [],
  };
  const nodes = runNodes(source);
  return {
    id: graphId(subject.name, item),
    subject: subject.name,
    item,
    run: subject.run,
    workflow: detail.definition.name,
    source: detail.definition.source,
    path: [project.name, subject.name, ...(item ? [item] : [])],
    nodes,
    edges: runEdges(source, nodes),
    steps: detail.definition.steps,
    defaultSelection: defaultSelection(nodes),
  };
}

export function previewGraph(project: ProjectView, name: string, source: string): RunGraph {
  const steps = definedSteps(parseToml(source));
  const subject: SubjectView = {
    name: `workflow:${name}`,
    run: 0,
    runs: 0,
    states: [],
    endedAt: null,
    driven: false,
    branch: null,
    itemStep: null,
    items: [],
    gates: [],
    blocked: [],
  };
  const graph = buildRunGraph(project, subject, null, { definition: { name, source, steps }, events: [] });
  return { ...graph, path: [project.name, `${name}.toml`], preview: true };
}

function defaultSelection(nodes: GraphNode[]) {
  const steps = nodes.filter((node) => node.type === "step");
  const state = (node: GraphNode) => (node.data as StepData).state;
  const attention = steps.find((node) => ATTENTION.includes(state(node)));
  const finished = steps.filter((node) => state(node) === "succeeded").at(-1);
  return (attention ?? finished ?? steps[0])?.id ?? "";
}
