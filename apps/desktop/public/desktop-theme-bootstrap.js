(() => {
  const storageKey = "lifetrace.app-preferences.v1";
  let preference = "system";

  try {
    const raw = window.localStorage.getItem(storageKey);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (parsed?.theme === "light" || parsed?.theme === "dark" || parsed?.theme === "system") {
        preference = parsed.theme;
      }
    }
  } catch {
    // A damaged preference cache must never block the desktop shell from loading.
  }

  const cookie = document.cookie
    .split("; ")
    .find((item) => item.startsWith("lifetrace_theme="));
  const cookieTheme = cookie?.slice("lifetrace_theme=".length);
  const cachedTheme = cookieTheme === "light" || cookieTheme === "dark" ? cookieTheme : null;
  const resolved = preference === "light" || preference === "dark"
    ? preference
    : cachedTheme ?? (window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light");

  document.documentElement.dataset.themePreference = preference;
  document.documentElement.dataset.theme = resolved;
  document.documentElement.style.colorScheme = resolved;
  document.querySelector('meta[name="theme-color"]')?.setAttribute(
    "content",
    resolved === "dark" ? "#171a18" : "#f4f5f4",
  );
})();
