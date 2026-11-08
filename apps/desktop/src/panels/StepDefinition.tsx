import { useMemo } from "react";
import { ArrowLeft, ArrowRight, FileCode2, FileText } from "lucide-react";
import { contractOf, list, table, text, type Flow } from "../engine/contracts";
import { useDefinitions } from "../editor/DefinitionsContext";
import { useWorkflowSpecs } from "../editor/useWorkflowSpecs";
import { useEngine } from "../engine/EngineContext";
import { useRunView } from "../graph/RunViewContext";
import { AgentAvatar } from "../ui/status";
import { DefSection } from "./DefSection";
import { StepSource } from "./StepSource";
import "./definition.css";

export function StepDefinition({ stepKey }: { stepKey: string }) {
  const { graph } = useRunView();
  const workflow = useWorkflowSpecs(graph?.workflow ?? "", graph?.source ?? "", Boolean(graph?.item));
  const contract = useMemo(() => contractOf(workflow.specs, stepKey), [workflow.specs, stepKey]);
  const raw = table(table(workflow.specs.workflow.steps)[stepKey]);
  const agent = text(raw.agent);
  if (!graph) return null;
  return (
    <div className="def">
      <p className="def__summary">{contract.summary}</p>
      {agent ? <AgentCard id={agent} profile={table(workflow.specs.agents[agent]?.profile)} /> : null}
      {text(raw.kind) === "loop" ? (
        <LoopBody raw={raw} />
      ) : (
        <>
          <FlowSection title="Inputs" flows={contract.inputs} direction="in" empty="Reads no artifacts." />
          <FlowSection title="Outputs" flows={contract.outputs} direction="out" empty="Produces no artifacts." />
        </>
      )}
      {contract.outcomes.length ? (
        <DefSection title="Outcomes">
          <div className="def__chips">
            {contract.outcomes.map((outcome) => (
              <span key={outcome} className="def-chip mono">
                {outcome}
              </span>
            ))}
          </div>
        </DefSection>
      ) : null}
      <Order raw={raw} />
      <StepSource stepKey={stepKey} workflow={graph.workflow} file={workflow.file} live={workflow.live} frozen={graph.source} run={graph.run} />
    </div>
  );
}

function FlowSection({ title, flows, direction, empty }: { title: string; flows: Flow[]; direction: "in" | "out"; empty: string }) {
  return (
    <DefSection title={title} aside={<span className="def-section__count mono">{flows.length}</span>}>
      {flows.length ? (
        <ul className="def-flows">
          {flows.map((flow) => (
            <FlowRow key={flow.name} flow={flow} direction={direction} />
          ))}
        </ul>
      ) : (
        <p className="def__empty">{empty}</p>
      )}
    </DefSection>
  );
}

function FlowRow({ flow, direction }: { flow: Flow; direction: "in" | "out" }) {
  const Arrow = direction === "in" ? ArrowLeft : ArrowRight;
  return (
    <li className={`def-flow def-flow--${direction}`}>
      <div className="def-flow__line">
        <span className="def-flow__name mono">{flow.name}</span>
        {flow.format ? <span className="def-flow__format mono">{flow.format}</span> : null}
        {flow.optional ? <span className="def-flow__optional">optional</span> : null}
      </div>
      <div className="def-flow__peers">
        <Arrow size={11} />
        {flow.peers.length ? <span>{direction === "in" ? "from" : "to"}</span> : null}
        {flow.peers.map((peer) => (
          <Peer key={peer} id={peer} />
        ))}
        {!flow.peers.length && !flow.note ? <span className="def-flow__none">{missing(direction, flow.optional)}</span> : null}
        {flow.note ? <span className="def-flow__note">{flow.peers.length ? `· ${flow.note}` : flow.note}</span> : null}
      </div>
    </li>
  );
}

function missing(direction: "in" | "out", optional?: boolean) {
  if (direction === "out") return "no step in this flow reads it";
  return optional ? "not produced in this flow" : "nothing in this flow produces it";
}

function Peer({ id }: { id: string }) {
  const { graph, select } = useRunView();
  const present = graph?.nodes.some((node) => node.id === id);
  if (!present) return <span className="def-peer def-peer--plain">{id}</span>;
  return (
    <button className="def-peer" onClick={() => select(id)} title={`Select ${id}`}>
      {id}
    </button>
  );
}

function LoopBody({ raw }: { raw: ReturnType<typeof table> }) {
  const until = table(raw.until);
  return (
    <DefSection title="Body">
      <div className="def-order">
        <span className="def-order__label">repeats</span>
        <span className="def-order__value">
          {list(raw.body).map((member) => (
            <Peer key={member} id={member} />
          ))}
        </span>
      </div>
      <div className="def-order">
        <span className="def-order__label">exits when</span>
        <span className="def-order__value mono">
          {text(until.step)} = {text(until.outcome)}
        </span>
      </div>
    </DefSection>
  );
}

function Order({ raw }: { raw: ReturnType<typeof table> }) {
  const needs = list(raw.needs);
  const when = table(raw.when);
  if (!needs.length && !text(when.step)) return null;
  return (
    <DefSection title="Order">
      {needs.length ? (
        <div className="def-order">
          <span className="def-order__label">runs after</span>
          <span className="def-order__value">
            {needs.map((need) => (
              <Peer key={need} id={need} />
            ))}
          </span>
        </div>
      ) : null}
      {text(when.step) ? (
        <div className="def-order">
          <span className="def-order__label">runs when</span>
          <span className="def-order__value mono">
            {text(when.step)} = {text(when.outcome)}
          </span>
        </div>
      ) : null}
    </DefSection>
  );
}

function AgentCard({ id, profile }: { id: string; profile: ReturnType<typeof table> }) {
  const { edit } = useDefinitions();
  const { available } = useEngine();
  const facts = ["model", "runtime", "effort", "conversation"].map((key) => [key, text(profile[key])] as const).filter(([, value]) => value);
  return (
    <DefSection title="Agent">
      <div className="def-agent">
        <AgentAvatar agent={id} size={26} />
        <span className="def-agent__name">
          {id}
          <span className="mono def-agent__path">.surt/agents/{id}</span>
        </span>
      </div>
      {facts.length ? (
        <dl className="def-facts">
          {facts.map(([key, value]) => (
            <div key={key}>
              <dt>{key}</dt>
              <dd className="mono">{value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
      {available ? (
        <div className="def-actions">
          <button className="btn btn-ghost" onClick={() => edit({ kind: "agent", id, file: "instructions.md" })}>
            <FileText size={12} /> Instructions
          </button>
          <button className="btn btn-ghost" onClick={() => edit({ kind: "agent", id, file: "agent.toml" })}>
            <FileCode2 size={12} /> agent.toml
          </button>
        </div>
      ) : null}
    </DefSection>
  );
}
