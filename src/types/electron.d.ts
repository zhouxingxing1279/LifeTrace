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
    vaultApi?: {
      status():Promise<VaultStatus>;
      initialize(password:string):Promise<VaultStatus>;
      unlock(password:string):Promise<VaultStatus>;
      lock():Promise<VaultStatus>;
      listAssets(options?:{trashed?:boolean;albumId?:string|null}):Promise<VaultAsset[]>;
      listAlbums():Promise<VaultAlbum[]>;
      importFiles(options?:{moveSource?:boolean;albumId?:string|null}):Promise<VaultAsset[]>;
      readAsset(assetId:string):Promise<VaultAssetPayload>;
      readThumbnail(assetId:string):Promise<VaultThumbnailPayload>;
      exportAsset(assetId:string,removeFromVault:boolean):Promise<string|null>;
      moveToTrash(assetId:string):Promise<void>;
      restoreAsset(assetId:string):Promise<void>;
      deleteAssetPermanently(assetId:string):Promise<void>;
      createAlbum(name:string):Promise<VaultAlbum>;
      renameAlbum(albumId:string,name:string):Promise<void>;
      deleteAlbum(albumId:string):Promise<void>;
      setAssetAlbum(assetId:string,albumId:string,assigned:boolean):Promise<void>;
      verifyIntegrity():Promise<VaultIntegrityReport>;
      changePassword(oldPassword:string,newPassword:string):Promise<VaultStatus>;
      setAutoLock(seconds:number):Promise<VaultStatus>;
      setLockOnBlur(enabled:boolean):Promise<VaultStatus>;
      deleteAll(confirmation:string):Promise<void>;
    };
  }

  type VaultStatus = {
    configured:boolean;
    unlocked:boolean;
    assetCount?:number;
    trashCount?:number;
    albumCount?:number;
    autoLockSeconds:number;
    lockOnBlur:boolean;
  };

  type VaultAssetState = "active" | "trash";

  type VaultAsset = {
    id:string;
    originalName:string;
    mimeType:string;
    size:number;
    importedAt:string;
    hasThumbnail:boolean;
    state:VaultAssetState;
    albumIds:string[];
    deletedAt?:string|null;
  };

  type VaultAlbum = {
    id:string;
    name:string;
    createdAt:string;
  };

  type VaultIntegrityReport = {
    checked:number;
    healthy:number;
    corruptedAssetIds:string[];
  };

  type VaultAssetPayload = {
    asset:VaultAsset;
    dataBase64:string;
  };

  type VaultThumbnailPayload = {
    assetId:string;
    mimeType:string;
    dataBase64:string;
  };

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
