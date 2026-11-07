import { createContext, useContext, type ReactNode } from "react";
import { PRESETS, type Choice } from "./options";

const ChoiceContext = createContext<Choice>(PRESETS.studio);

export function useChoice() {
  return useContext(ChoiceContext);
}

type SurfaceProps = {
  choice: Choice;
  children: ReactNode;
  className?: string;
};

export function ChoiceSurface({ choice, children, className }: SurfaceProps) {
  return (
    <ChoiceContext.Provider value={choice}>
      <div className={`surface ${className ?? ""}`} data-surface={choice.surface} data-states={choice.states}>
        {children}
      </div>
    </ChoiceContext.Provider>
  );
}
