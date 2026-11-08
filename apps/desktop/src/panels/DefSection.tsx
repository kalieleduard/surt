import type { ReactNode } from "react";

export function DefSection({ title, aside, children }: { title: string; aside?: ReactNode; children: ReactNode }) {
  return (
    <section className="def-section">
      <header className="def-section__head">
        <span>{title}</span>
        {aside}
      </header>
      {children}
    </section>
  );
}
