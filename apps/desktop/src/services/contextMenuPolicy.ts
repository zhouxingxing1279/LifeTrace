const editableSelector = [
  "input",
  "textarea",
  "[contenteditable='true']",
  ".ProseMirror",
].join(",");

/**
 * Desktop WebView policy:
 * - custom app context menus keep working because they prevent/stop the event first;
 * - editable controls retain the useful native edit menu;
 * - everywhere else the browser/WebView context menu is suppressed.
 */
export function installDesktopContextMenuPolicy() {
  const handleContextMenu = (event: MouseEvent) => {
    if (event.defaultPrevented) return;

    const target = event.target;
    if (target instanceof Element && target.closest(editableSelector)) return;

    event.preventDefault();
  };

  document.addEventListener("contextmenu", handleContextMenu);
  return () => document.removeEventListener("contextmenu", handleContextMenu);
}
