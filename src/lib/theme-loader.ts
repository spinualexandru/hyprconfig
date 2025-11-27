import { invoke } from "@tauri-apps/api/core";

const THEME_STYLE_ID = "hyprconfig-theme";

/**
 * Load theme CSS from the config file and inject it into the document head.
 * Creates the default theme file if it doesn't exist.
 */
export async function loadTheme(): Promise<void> {
  try {
    const css = await invoke<string>("get_theme_css");
    injectThemeCSS(css);
  } catch (error) {
    console.error("Failed to load theme CSS:", error);
  }
}

/**
 * Reload the theme CSS from disk.
 * Useful after matugen generates a new theme.
 */
export async function reloadTheme(): Promise<void> {
  return loadTheme();
}

/**
 * Inject CSS into the document head, replacing any existing theme styles.
 */
function injectThemeCSS(css: string): void {
  let styleElement = document.getElementById(THEME_STYLE_ID) as HTMLStyleElement | null;

  if (!styleElement) {
    styleElement = document.createElement("style");
    styleElement.id = THEME_STYLE_ID;
    document.head.appendChild(styleElement);
  }

  styleElement.textContent = css;
}

/**
 * Ensure the matugen template file exists.
 * Returns the path to the template file.
 */
export async function ensureMatugenTemplate(): Promise<string> {
  return invoke<string>("ensure_matugen_template");
}
