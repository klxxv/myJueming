export type SystemTheme = "light" | "dark";
type Unlisten = () => void;

export interface ThemeHost {
  theme(): Promise<SystemTheme | null>;
  onThemeChanged(handler: (event: { payload: SystemTheme }) => void): Promise<Unlisten>;
}

// Subscribe before reading, so a change during the async read wins over its snapshot.
export async function observeNativeTheme(host: ThemeHost, notify: (theme: SystemTheme) => void): Promise<Unlisten> {
  let active = true;
  let generation = 0;
  const unlisten = await host.onThemeChanged(({ payload }) => {
    generation++;
    if (active) notify(payload);
  });
  const refresh = async () => {
    const request = ++generation;
    try {
      const theme = await host.theme();
      if (active && request === generation && theme) notify(theme);
    } catch {
      // Keep the last native value (or media-query fallback) if the host is unavailable.
    }
  };
  const onFocus = () => { void refresh(); };
  window.addEventListener("focus", onFocus);
  await refresh();
  return () => {
    active = false;
    generation++;
    window.removeEventListener("focus", onFocus);
    unlisten();
  };
}
