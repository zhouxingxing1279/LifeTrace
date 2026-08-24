(() => {
  const cookie = document.cookie
    .split("; ")
    .find((item) => item.startsWith("lifetrace_theme="));
  const theme = cookie?.slice("lifetrace_theme=".length);
  if (theme !== "light" && theme !== "dark") return;

  document.documentElement.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme;
  document.querySelector('meta[name="theme-color"]')?.setAttribute(
    "content",
    theme === "dark" ? "#101613" : "#f4f6f4",
  );
})();
