const SYMBOL_PATH = "M172 4 22 79v42l102 39L4 256l168-78v-37L70 102Z";

type SurtMarkProps = {
  size?: number;
  color?: string;
};

export function SurtSymbol({ size = 18, color = "var(--surt-lava)" }: SurtMarkProps) {
  return (
    <svg width={(size * 180) / 260} height={size} viewBox="0 0 180 260" aria-hidden="true">
      <path d={SYMBOL_PATH} fill={color} />
    </svg>
  );
}

export function SurtWordmark({ size = 18 }: { size?: number }) {
  return (
    <span className="surt-wordmark" style={{ fontSize: size }}>
      <SurtSymbol size={size * 1.05} />
      <span>Surt</span>
    </span>
  );
}
