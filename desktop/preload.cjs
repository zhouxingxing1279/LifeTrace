const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("noteApi", {
  selectAttachment: (noteId) => ipcRenderer.invoke("notes:select-attachment", { noteId }),
  openAttachment: (noteId, fileName) => ipcRenderer.invoke("notes:open-attachment", { noteId, fileName }),
  showAttachment: (noteId, fileName) => ipcRenderer.invoke("notes:show-attachment", { noteId, fileName }),
  deleteAttachment: (noteId, fileName) => ipcRenderer.invoke("notes:delete-attachment", { noteId, fileName }),
  exportNote: (payload) => ipcRenderer.invoke("notes:export", payload),
  importMarkdown: () => ipcRenderer.invoke("notes:import-markdown"),
  onCommand: (listener) => {
    const receive = (_event, command) => listener(command);
    ipcRenderer.on("notes:command", receive);
    return () => ipcRenderer.removeListener("notes:command", receive);
  },
});
