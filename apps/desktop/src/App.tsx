import { ThemeProvider } from "./theme/ThemeContext";
import { Workspace } from "./workspace/Workspace";

export function App() {
  return (
    <ThemeProvider>
      <Workspace />
    </ThemeProvider>
  );
}
