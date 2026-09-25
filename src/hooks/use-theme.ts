import { useCallback, useEffect, useSyncExternalStore } from "react";

type Theme = "light" | "dark";

// Module-level store so every consumer (sidebar toggle, toasts) shares one theme
const listeners = new Set<() => void>();
// Default to light mode
let currentTheme: Theme =
	localStorage.getItem("theme") === "dark" ? "dark" : "light";

function subscribe(listener: () => void) {
	listeners.add(listener);
	return () => {
		listeners.delete(listener);
	};
}

function getSnapshot() {
	return currentTheme;
}

function setTheme(theme: Theme) {
	currentTheme = theme;
	localStorage.setItem("theme", theme);
	for (const listener of listeners) {
		listener();
	}
}

export function useTheme() {
	const theme = useSyncExternalStore(subscribe, getSnapshot);

	useEffect(() => {
		document.documentElement.classList.toggle("dark", theme === "dark");
	}, [theme]);

	const toggleTheme = useCallback(() => {
		setTheme(currentTheme === "light" ? "dark" : "light");
	}, []);

	return { theme, toggleTheme };
}
