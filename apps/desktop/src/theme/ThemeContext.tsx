import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

export type Theme = "obsidiana" | "grafite" | "mineral";

export const THEMES: { value: Theme; label: string; hint: string; swatch: string }[] = [
  { value: "obsidiana", label: "Obsidiana", hint: "Darkest, the brand base", swatch: "#0b0c0d" },
  { value: "grafite", label: "Grafite", hint: "Dark with raised panels", swatch: "#242628" },
  { value: "mineral", label: "Mineral", hint: "Light", swatch: "#f4f1ea" },
];

const DEFAULT_THEME: Theme = "grafite";
const STORAGE_KEY = "surt-theme";

type ThemeState = { theme: Theme; setTheme: (theme: Theme) => void };

const ThemeContext = createContext<ThemeState>({ theme: DEFAULT_THEME, setTheme: () => undefined });

export function useTheme() {
  return useContext(ThemeContext);
}

function isTheme(value: string | null): value is Theme {
  return THEMES.some((theme) => theme.value === value);
}

function storedTheme(): Theme {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return isTheme(value) ? value : DEFAULT_THEME;
  } catch {
    return DEFAULT_THEME;
  }
}

function storeTheme(theme: Theme) {
  try {
    localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    return;
  }
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState<Theme>(storedTheme);

  useEffect(() => storeTheme(theme), [theme]);

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>
      <div className="surface" data-surface={theme}>
        {children}
      </div>
    </ThemeContext.Provider>
  );
}
