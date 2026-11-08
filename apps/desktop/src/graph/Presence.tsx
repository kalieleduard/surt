export function Presence({ agent }: { agent: string }) {
  return (
    <span className={`presence presence--${agent}`}>
      <svg width="14" height="16" viewBox="0 0 14 16" aria-hidden="true">
        <path d="M1 1l11 6.2-5 1.3L4.6 14z" />
      </svg>
      <span className="presence__label">{agent}</span>
    </span>
  );
}
