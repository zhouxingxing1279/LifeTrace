export {};

declare global {
  interface Window {
    noteApi?: {
      selectAttachment(noteId:string):Promise<{ok:boolean;canceled?:boolean;error?:string;file?:Record<string,unknown>}>;
      openAttachment(noteId:string,fileName:string):Promise<{ok:boolean;error?:string}>;
      showAttachment(noteId:string,fileName:string):Promise<{ok:boolean;error?:string}>;
      deleteAttachment(noteId:string,fileName:string):Promise<{ok:boolean;error?:string}>;
      exportNote(payload:{format:"md"|"html"|"json";title:string;content:string}):Promise<{ok:boolean;canceled?:boolean;error?:string;filePath?:string}>;
      importMarkdown():Promise<{ok:boolean;canceled?:boolean;error?:string;title?:string;content?:string}>;
      onCommand(listener:(command:string)=>void):()=>void;
    };
  }
}
