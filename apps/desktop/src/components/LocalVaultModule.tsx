"use client";
/* eslint-disable @next/next/no-img-element -- decrypted local object URLs are not compatible with the image optimizer */

import { FormEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle, ArchiveRestore, Eye, FileLock2, FolderPlus,
  Image as ImageIcon, KeyRound, LoaderCircle, LockKeyhole, LogOut, Pencil, ShieldCheck,
  Trash2, X,
} from "lucide-react";
import Toast from "@/src/components/Toast";

const DELETE_CONFIRMATION = "永久删除私密相册";

const formatBytes = (value:number) => new Intl.NumberFormat("zh-CN", {
  style:"unit", unit:value >= 1024 ** 2 ? "megabyte" : "kilobyte", maximumFractionDigits:1,
}).format(value / (value >= 1024 ** 2 ? 1024 ** 2 : 1024));

function base64ObjectUrl(dataBase64:string,mimeType:string) {
  const binary = window.atob(dataBase64);
  const bytes = new Uint8Array(binary.length);
  for (let index=0; index<binary.length; index+=1) bytes[index]=binary.charCodeAt(index);
  return URL.createObjectURL(new Blob([bytes],{type:mimeType}));
}

export default function LocalVaultModule() {
  const api=typeof window!=="undefined"?window.vaultApi:undefined;
  const [status,setStatus]=useState<VaultStatus|null>(null);
  const [assets,setAssets]=useState<VaultAsset[]>([]);
  const [albums,setAlbums]=useState<VaultAlbum[]>([]);
  const [view,setView]=useState<"active"|"trash">("active");
  const [albumId,setAlbumId]=useState<string|null>(null);
  const [thumbnails,setThumbnails]=useState<Record<string,string>>({});
  const [preview,setPreview]=useState<{asset:VaultAsset;url:string}|null>(null);
  const [loading,setLoading]=useState(true);
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState("");
  const [message,setMessage]=useState("");
  const [password,setPassword]=useState("");
  const [confirmation,setConfirmation]=useState("");
  const [setupPassword,setSetupPassword]=useState("");
  const [setupRepeat,setSetupRepeat]=useState("");
  const [setupAccepted,setSetupAccepted]=useState(false);
  const [showPasswordForm,setShowPasswordForm]=useState(false);
  const [oldPassword,setOldPassword]=useState("");
  const [newPassword,setNewPassword]=useState("");
  const [repeatPassword,setRepeatPassword]=useState("");
  const [contextMenu,setContextMenu]=useState<{x:number;y:number;asset:VaultAsset}|null>(null);
  const objectUrls=useRef(new Set<string>());

  const closeContextMenu=useCallback(()=>setContextMenu(null),[]);
  const openContextMenu=(event:React.MouseEvent,asset:VaultAsset)=>{
    event.preventDefault();
    event.stopPropagation();
    setContextMenu({x:event.clientX,y:event.clientY,asset});
  };
  useEffect(()=>{
    if(!contextMenu)return;
    const close=()=>closeContextMenu();
    const onKey=(event:KeyboardEvent)=>{if(event.key==="Escape")closeContextMenu()};
    window.addEventListener("click",close);
    window.addEventListener("contextmenu",close);
    window.addEventListener("scroll",close,true);
    window.addEventListener("keydown",onKey);
    return()=>{
      window.removeEventListener("click",close);
      window.removeEventListener("contextmenu",close);
      window.removeEventListener("scroll",close,true);
      window.removeEventListener("keydown",onKey);
    };
  },[contextMenu,closeContextMenu]);

  const trackUrl=useCallback((url:string)=>{objectUrls.current.add(url);return url},[]);
  const clearSensitive=useCallback(()=>{
    for(const url of objectUrls.current)URL.revokeObjectURL(url);
    objectUrls.current.clear();
    setThumbnails({});setPreview(null);setAssets([]);setAlbums([]);setAlbumId(null);
  },[]);

  const loadStatus=useCallback(async()=>{
    if(!api)return;
    const next=await api.status();setStatus(next);
  },[api]);

  const refresh=useCallback(async()=>{
    if(!api||!status?.unlocked)return;
    const [nextAssets,nextAlbums]=await Promise.all([
      api.listAssets({trashed:view==="trash",albumId:view==="active"?albumId:null}),
      api.listAlbums(),
    ]);
    setAssets(nextAssets);setAlbums(nextAlbums);
    const nextThumbs:Record<string,string>={};
    await Promise.all(nextAssets.filter(asset=>asset.hasThumbnail).map(async asset=>{
      try{
        const payload=await api.readThumbnail(asset.id);
        nextThumbs[asset.id]=trackUrl(base64ObjectUrl(payload.dataBase64,payload.mimeType));
      }catch{/* A damaged thumbnail is reported by integrity check; keep the placeholder here. */}
    }));
    setThumbnails(current=>{
      for(const url of Object.values(current) as string[]){URL.revokeObjectURL(url);objectUrls.current.delete(url)}
      return nextThumbs;
    });
    setStatus(await api.status());
  },[albumId,api,status?.unlocked,trackUrl,view]);

  useEffect(()=>{void (async()=>{try{await loadStatus()}catch(cause){setError(String(cause))}finally{setLoading(false)}})()},[loadStatus]);
  useEffect(()=>{if(status?.unlocked)void refresh()},[refresh,status?.unlocked]);
  useEffect(()=>()=>clearSensitive(),[clearSensitive]);

  const lock=useCallback(async()=>{
    if(!api)return;
    try{setStatus(await api.lock())}finally{clearSensitive();setPassword("");setShowPasswordForm(false)}
  },[api,clearSensitive]);

  useEffect(()=>{
    if(!status?.unlocked||!status.lockOnBlur||busy)return;
    const hide=()=>{if(document.visibilityState==="hidden")void lock()};
    const blur=()=>void lock();
    window.addEventListener("blur",blur);document.addEventListener("visibilitychange",hide);
    return()=>{window.removeEventListener("blur",blur);document.removeEventListener("visibilitychange",hide)};
  },[busy,lock,status?.lockOnBlur,status?.unlocked]);

  const perform=useCallback(async(task:()=>Promise<void>,success?:string)=>{
    setBusy(true);setError("");setMessage("");
    try{await task();if(success)setMessage(success)}catch(cause){setError(cause instanceof Error?cause.message:String(cause))}
    finally{setBusy(false)}
  },[]);

  const setup=async(event:FormEvent)=>{event.preventDefault();if(!api)return;
    await perform(async()=>{
      if(!setupAccepted)throw new Error("请先确认密码丢失后无法恢复");
      if(setupPassword!==setupRepeat)throw new Error("两次输入的密码不一致");
      setStatus(await api.initialize(setupPassword));setSetupPassword("");setSetupRepeat("");
    },"私密相册已创建，所有内容只保存在当前电脑。")
  };
  const unlock=async(event:FormEvent)=>{event.preventDefault();if(!api)return;
    await perform(async()=>{setStatus(await api.unlock(password));setPassword("")})
  };
  const openPreview=(asset:VaultAsset)=>perform(async()=>{
    if(!api)return;const payload=await api.readAsset(asset.id);
    if(preview){URL.revokeObjectURL(preview.url);objectUrls.current.delete(preview.url)}
    setPreview({asset:payload.asset,url:trackUrl(base64ObjectUrl(payload.dataBase64,payload.asset.mimeType))});
  });
  const closePreview=()=>{if(preview){URL.revokeObjectURL(preview.url);objectUrls.current.delete(preview.url)}setPreview(null)};
  const moveToTrash=(asset:VaultAsset)=>perform(async()=>{if(!api)return;await api.moveToTrash(asset.id);await refresh()},"已移入私密回收站。");
  const restore=(asset:VaultAsset)=>perform(async()=>{if(!api)return;await api.restoreAsset(asset.id);await refresh()},"已恢复到私密相册。");
  const restoreToSyncAlbum=async(asset:VaultAsset)=>{
    if(!api)return;
    setError("");setMessage("正在恢复到同步相册，请稍候…");
    try{
      await api.restoreToSyncAlbum(asset.id);
      closePreview();await refresh();
      setMessage("已恢复到同步相册。");
    }catch(cause){
      setError(cause instanceof Error?cause.message:String(cause));
    }
  };
  const permanentDelete=(asset:VaultAsset)=>{
    if(!window.confirm(`永久删除“${asset.originalName}”？此操作无法撤销。`))return;
    void perform(async()=>{if(!api)return;await api.deleteAssetPermanently(asset.id);await refresh()},"已永久删除密文和索引记录。");
  };
  const createAlbum=()=>{const name=window.prompt("新建私密子相册名称");if(!name)return;
    void perform(async()=>{if(!api)return;const album=await api.createAlbum(name);setAlbumId(album.id);await refresh()},"私密子相册已创建。")};
  const renameAlbum=()=>{const album=albums.find(item=>item.id===albumId);if(!album)return;const name=window.prompt("重命名私密子相册",album.name);if(!name)return;
    void perform(async()=>{if(!api)return;await api.renameAlbum(album.id,name);await refresh()},"子相册已重命名。")};
  const deleteAlbum=()=>{const album=albums.find(item=>item.id===albumId);if(!album||!window.confirm(`删除子相册“${album.name}”？照片不会被删除。`))return;
    void perform(async()=>{if(!api)return;await api.deleteAlbum(album.id);setAlbumId(null);await refresh()},"子相册已删除，照片仍保留在私密相册。")};
  const assignAlbum=(asset:VaultAsset,nextAlbumId:string)=>{if(!nextAlbumId)return;
    void perform(async()=>{if(!api)return;await api.setAssetAlbum(asset.id,nextAlbumId,true);await refresh()},"照片已加入子相册。")};
  const removeFromCurrentAlbum=(asset:VaultAsset)=>{if(!albumId)return;
    void perform(async()=>{if(!api)return;await api.setAssetAlbum(asset.id,albumId,false);await refresh()},"照片已移出当前子相册。")};
  const verify=()=>perform(async()=>{if(!api)return;const report=await api.verifyIntegrity();
    setMessage(report.corruptedAssetIds.length===0?`完整性检查通过：${report.healthy}/${report.checked} 项健康。`:`发现 ${report.corruptedAssetIds.length} 个损坏对象，请勿删除现有备份。`)
  });
  const changePassword=async(event:FormEvent)=>{event.preventDefault();if(!api)return;
    await perform(async()=>{if(newPassword!==repeatPassword)throw new Error("两次输入的新密码不一致");setStatus(await api.changePassword(oldPassword,newPassword));setOldPassword("");setNewPassword("");setRepeatPassword("");setShowPasswordForm(false)},"密码已修改，照片文件无需重新加密。")
  };
  const deleteAll=()=>{if(!api||confirmation!==DELETE_CONFIRMATION)return;
    void perform(async()=>{await api.deleteAll(confirmation);clearSensitive();setStatus(await api.status());setConfirmation("")},"私密相册已永久删除。")};

  const currentAlbum=useMemo(()=>albums.find(album=>album.id===albumId)??null,[albumId,albums]);
  if(!api)return <div className="local-vault unavailable"><LockKeyhole/><h3>私密相册仅在 LifeTrace 桌面端提供</h3><p>该模块不会通过网页或局域网接口开放。</p></div>;
  if(loading||!status)return <div className="local-vault loading"><LoaderCircle className="spin"/><p>正在检查本地加密空间…</p></div>;

  if(!status.configured)return <div className="local-vault vault-gate"><form className="vault-gate-card" onSubmit={setup}>
    {(message||error)&&<Toast kind={error?"error":"info"} message={error||message} onClose={()=>{setError("");setMessage("")}}/>}
    <span className="vault-gate-icon"><FileLock2/></span><p className="vault-kicker">仅本机 · 零恢复</p><h2>创建私密相册</h2>
    <p>原图、视频、缩略图和名称都会在当前电脑上加密，不会进入同步、分享或任何云端服务。</p>
    <div className="vault-warning"><AlertTriangle/><span>系统没有找回密码、恢复密钥或管理员后门。忘记密码后，只能永久删除整个私密相册。</span></div>
    <label>私密相册密码<input type="password" minLength={6} value={setupPassword} onChange={event=>setSetupPassword(event.target.value)} required autoComplete="new-password"/></label>
    <label>再次输入密码<input type="password" minLength={6} value={setupRepeat} onChange={event=>setSetupRepeat(event.target.value)} required autoComplete="new-password"/></label>
    <label className="vault-confirm"><input type="checkbox" checked={setupAccepted} onChange={event=>setSetupAccepted(event.target.checked)}/><span>我确认密码丢失后，任何人都无法恢复其中内容。</span></label>
    <button className="vault-primary" disabled={busy||!setupAccepted}><KeyRound/>创建本地加密空间</button>
  </form></div>;

  if(!status.unlocked)return <div className="local-vault vault-gate"><form className="vault-gate-card" onSubmit={unlock}>
    {(message||error)&&<Toast kind={error?"error":"info"} message={error||message} onClose={()=>{setError("");setMessage("")}}/>}
    <span className="vault-gate-icon"><LockKeyhole/></span><p className="vault-kicker">已锁定</p><h2>私密相册</h2>
    <p>锁定状态不加载数量、封面、文件名或任何私密元数据。</p>
    <label>密码<input type="password" value={password} onChange={event=>setPassword(event.target.value)} required autoFocus autoComplete="current-password"/></label>
    <button className="vault-primary" disabled={busy}><KeyRound/>解锁</button>
    <button type="button" className="vault-danger-link" onClick={()=>setConfirmation(current=>current?"":" ")}>忘记密码后永久删除私密相册</button>
    {confirmation!==""&&<div className="vault-danger-zone"><p>输入“{DELETE_CONFIRMATION}”后删除所有密文。此操作不需要密码，但无法撤销。</p><input value={confirmation.trimStart()} onChange={event=>setConfirmation(event.target.value)} placeholder={DELETE_CONFIRMATION}/><button type="button" className="vault-danger" disabled={confirmation.trim()!==DELETE_CONFIRMATION||busy} onClick={deleteAll}><Trash2/>永久删除</button></div>}
  </form></div>;

  return <div className="local-vault vault-open">
    {(message||error)&&<Toast kind={error?"error":"info"} message={error||message} onClose={()=>{setError("");setMessage("")}}/>}
    <header className="vault-header"><div><p className="vault-kicker">LOCAL ENCRYPTED STORAGE</p><h2><ShieldCheck/>私密相册</h2><p>仅本机加解密；与手机同步、云同步、分享链接和远程分析完全隔离。</p></div><div className="vault-actions">
      <button onClick={verify} disabled={busy}><ShieldCheck/>完整性检查</button><button className="vault-lock" onClick={()=>void lock()}><LogOut/>锁定</button>
    </div></header>
    <section className="vault-summary"><div><span>私密照片和视频</span><strong>{status.assetCount??0}</strong></div><div><span>回收站</span><strong>{status.trashCount??0}</strong></div><div><span>子相册</span><strong>{status.albumCount??0}</strong></div>
      <label>空闲自动锁定<select value={status.autoLockSeconds} onChange={event=>void perform(async()=>setStatus(await api.setAutoLock(Number(event.target.value))))}><option value={30}>30 秒</option><option value={60}>1 分钟</option><option value={300}>5 分钟</option><option value={600}>10 分钟</option><option value={1800}>30 分钟</option></select></label>
      <label className="vault-switch"><input type="checkbox" checked={status.lockOnBlur} onChange={event=>void perform(async()=>setStatus(await api.setLockOnBlur(event.target.checked)))}/><span>离开窗口立即锁定</span></label>
    </section>
    <div className="vault-workspace"><aside className="vault-sidebar"><div className="vault-sidebar-head"><strong>浏览</strong><button onClick={createAlbum}><FolderPlus/></button></div>
      <button className={view==="active"&&!albumId?"active":""} onClick={()=>{setView("active");setAlbumId(null)}}>全部私密内容</button>
      {albums.map(album=><button key={album.id} className={view==="active"&&albumId===album.id?"active":""} onClick={()=>{setView("active");setAlbumId(album.id)}}>{album.name}</button>)}
      <button className={view==="trash"?"active danger":"danger"} onClick={()=>{setView("trash");setAlbumId(null)}}><Trash2/>私密回收站</button>
      {currentAlbum&&<div className="vault-album-actions"><button onClick={renameAlbum}><Pencil/>重命名</button><button onClick={deleteAlbum}><Trash2/>删除子相册</button></div>}
    </aside><main className="vault-content"><div className="vault-content-head"><div><span>{view==="trash"?"回收站":currentAlbum?.name||"全部内容"}</span><strong>{assets.length} 项</strong></div>{busy&&<LoaderCircle className="spin"/>}</div>
      {assets.length===0?<div className="vault-empty"><ImageIcon/><h3>{view==="trash"?"回收站为空":"还没有私密内容"}</h3><p>{view==="trash"?"删除的内容会先进入这里。":"在“同步相册”中选择照片并点击“批量隐藏”，照片就会加密移入这里。"}</p></div>:
      <div className="vault-grid">{assets.map(asset=><article className="vault-card" key={asset.id} onContextMenu={event=>openContextMenu(event,asset)}><button className="vault-card-preview" onClick={()=>openPreview(asset)}>{thumbnails[asset.id]?<img src={thumbnails[asset.id]} alt=""/>:<ImageIcon/>}{asset.mimeType.startsWith("video/")&&<span>视频</span>}</button>
        <div className="vault-card-copy"><strong title={asset.originalName}>{asset.originalName}</strong><small>{formatBytes(asset.size)} · {new Date(asset.importedAt).toLocaleString("zh-CN",{hour12:false})}</small></div>
      </article>)}</div>}
    </main></div>
    <section className="vault-settings"><button onClick={()=>setShowPasswordForm(current=>!current)}><KeyRound/>修改密码</button>{showPasswordForm&&<form className="vault-inline-form" onSubmit={changePassword}><h3>修改私密相册密码</h3><p>必须验证旧密码；只重新加密主密钥，不重新处理所有照片。</p><input type="password" placeholder="当前密码" value={oldPassword} onChange={event=>setOldPassword(event.target.value)} required/><input type="password" placeholder="新密码（至少 6 个字符）" value={newPassword} onChange={event=>setNewPassword(event.target.value)} required minLength={6}/><input type="password" placeholder="再次输入新密码" value={repeatPassword} onChange={event=>setRepeatPassword(event.target.value)} required minLength={6}/><div><button type="button" onClick={()=>setShowPasswordForm(false)}>取消</button><button className="vault-primary" disabled={busy}>确认修改</button></div></form>}</section>
    {preview&&<div className="vault-preview-modal" role="dialog" aria-modal="true" aria-label={preview.asset.originalName} onMouseDown={event=>{if(event.target===event.currentTarget)closePreview()}}><div className="vault-preview-panel"><header><div><strong>{preview.asset.originalName}</strong><small>{formatBytes(preview.asset.size)}</small></div><button onClick={closePreview}><X/></button></header>{preview.asset.mimeType.startsWith("video/")?<video src={preview.url} controls autoPlay/>:<img src={preview.url} alt={preview.asset.originalName}/>}</div></div>}
    {contextMenu&&<div className="vault-context-menu" role="menu" style={{left:Math.min(contextMenu.x,window.innerWidth-200),top:Math.min(contextMenu.y,window.innerHeight-240)}} onClick={event=>event.stopPropagation()} onContextMenu={event=>event.preventDefault()}>
      {view==="active"?<>
        <button role="menuitem" onClick={()=>{openPreview(contextMenu.asset);closeContextMenu()}}><Eye/>预览</button>
        <button role="menuitem" onClick={()=>{void restoreToSyncAlbum(contextMenu.asset);closeContextMenu()}}><ArchiveRestore/>恢复到同步相册</button>
        <button role="menuitem" onClick={()=>{void moveToTrash(contextMenu.asset);closeContextMenu()}}><Trash2/>移入回收站</button>
        {albums.filter(album=>!contextMenu.asset.albumIds.includes(album.id)).map(album=><button key={album.id} role="menuitem" onClick={()=>{void assignAlbum(contextMenu.asset,album.id);closeContextMenu()}}><FolderPlus/>加入「{album.name}」</button>)}
        {albumId&&<button role="menuitem" onClick={()=>{void removeFromCurrentAlbum(contextMenu.asset);closeContextMenu()}}><FolderPlus/>移出当前子相册</button>}
      </>:<>
        <button role="menuitem" onClick={()=>{void restore(contextMenu.asset);closeContextMenu()}}><ArchiveRestore/>恢复</button>
        <button role="menuitem" className="danger" onClick={()=>{void permanentDelete(contextMenu.asset);closeContextMenu()}}><Trash2/>永久删除</button>
      </>}
    </div>}
  </div>;
}
