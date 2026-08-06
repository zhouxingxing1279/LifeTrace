export {};

declare global {
  interface Window {
    syncApi?: {
      setSession(origin:string,accessToken:string,deviceId:string):Promise<import("@/src/services/cloudSync").SessionBindingResult>;
      clearSession():Promise<void>;
      bindCurrentProfile():Promise<string>;
      createCloudProfile(displayName:string):Promise<string>;
      profiles():Promise<import("@/src/services/cloudSync").LocalProfile[]>;
      setActiveProfile(profileId:string):Promise<void>;
      status():Promise<import("@/src/services/cloudSync").SyncStatusView>;
      now(forceSnapshot?:boolean):Promise<import("@/src/services/cloudSync").SyncRunReport>;
      conflicts():Promise<import("@/src/services/cloudSync").SyncConflictView[]>;
      resolveConflict(conflictId:string,resolution:"accept_remote"|"keep_local"|"discard"):Promise<void>;
    };
    cloudCredentialApi?: {
      set(refreshToken:string):Promise<void>;
      get():Promise<string|null>;
      clear():Promise<void>;
    };
    noteApi?: {
      selectAttachment(noteId:string):Promise<{ok:boolean;canceled?:boolean;error?:string;file?:Record<string,unknown>}>;
      openAttachment(noteId:string,fileName:string):Promise<{ok:boolean;error?:string}>;
      showAttachment(noteId:string,fileName:string):Promise<{ok:boolean;error?:string}>;
      deleteAttachment(noteId:string,fileName:string):Promise<{ok:boolean;error?:string}>;
      exportNote(payload:{format:"md"|"html"|"json";title:string;content:string}):Promise<{ok:boolean;canceled?:boolean;error?:string;filePath?:string}>;
      importMarkdown():Promise<{ok:boolean;canceled?:boolean;error?:string;title?:string;content?:string}>;
      onCommand(listener:(command:string)=>void):()=>void;
    };
    mobileUploadApi?: {
      status():Promise<MobileUploadResponse>;
      start():Promise<MobileUploadResponse>;
      stop():Promise<MobileUploadResponse>;
    };
    photoSyncApi?: {
      status():Promise<PhotoSyncDesktopResponse>;
      createPairing():Promise<PhotoSyncDesktopResponse>;
      cancelPairing(pairCode:string):Promise<PhotoSyncDesktopResponse>;
      recover():Promise<PhotoSyncDesktopResponse>;
      exportCertificate():Promise<PhotoSyncDesktopResponse>;
      setCompatibilityMode(enabled:boolean,confirmed?:boolean):Promise<PhotoSyncDesktopResponse>;
    };
  }

  type MobileUploadStatus = {
    available:boolean;
    active:boolean;
    managed:boolean;
    port:number;
    urls:string[];
    photoSyncUrls?:string[];
    computerName?:string;
    bindAddress?:string;
    mediaUrl?:string;
    certificateReady?:boolean;
    certificateAddresses?:string[];
    certificateExported?:boolean;
    certificateExportPath?:string;
    certificateCommonName?:string;
    allowInsecureHttp?:boolean;
    transportProtocol?:"http"|"https";
  };

  type MobileUploadResponse = {
    ok:boolean;
    status?:MobileUploadStatus;
    error?:string;
  };

  type PhotoSyncPairing = {
    success:boolean;
    pairCode:string;
    expiresAt:string;
    entryUrl:string;
  };

  type PhotoSyncDesktopResponse = {
    ok:boolean;
    status?:MobileUploadStatus & { pairing?:PhotoSyncPairing };
    error?:string;
  };
}
