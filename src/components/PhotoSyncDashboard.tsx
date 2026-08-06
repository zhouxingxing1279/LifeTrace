"use client";
/* eslint-disable @next/next/no-img-element -- local authenticated media URLs are not compatible with the image optimizer */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import QRCode from "qrcode";
import {
  AlertTriangle, CheckCircle2, Copy, EyeOff, Film, Image as ImageIcon, LockKeyhole,
  LoaderCircle, RefreshCw, Smartphone, X,
} from "lucide-react";
import Toast from "@/src/components/Toast";

type Photo = {
  id:string; original_file_name:string; media_type:"image"|"video"; mime_type:string|null;
  file_size:number; width:number|null; height:number|null; duration_ms:number|null;
  captured_at:string|null; imported_at:string; processing_status:string;
  processing_error:string|null; device_name:string|null;
};
type Device = {
  id:string; device_name:string; device_type:string; status:"active"|"revoked";
  paired_at:string; last_seen_at:string|null; revoked_at:string|null;
};
type UploadTask = {
  id:string; original_file_name:string; expected_file_size:number; received_file_size:number;
  status:string; photo_id:string|null; updated_at:string; error_code:string|null; error_message:string|null;
};
type Dashboard = {
  photos:Photo[]; total:number; page:number; pageSize:number; devices:Device[];
  tasks:UploadTask[]; summary:{success_count?:number;duplicate_count?:number;failed_count?:number;processing_count?:number;last_sync_at?:string};
};
type Pairing = { pairCode:string; expiresAt:string; entryUrl:string };

const mediaBase = "http://127.0.0.1:3444/photo-sync/media";
const formatBytes = (value:number) => new Intl.NumberFormat("zh-CN", {
  style:"unit", unit:value >= 1024 ** 2 ? "megabyte" : "kilobyte", maximumFractionDigits:1,
}).format(value / (value >= 1024 ** 2 ? 1024 ** 2 : 1024));
const formatDateTime = (value?:string|null) => value ? new Date(value).toLocaleString("zh-CN", { hour12:false }) : "尚未记录";
const statusLabel:Record<string,string> = {
  created:"等待上传", uploading:"正在上传", uploaded:"等待完成", processing:"正在处理",
  completed:"已完成", failed:"失败", expired:"已过期", pending:"等待处理",
};

function StateIcon({ status }:{ status:string }) {
  if (status === "failed") return <AlertTriangle aria-hidden="true"/>;
  if (["processing","uploading","uploaded","pending"].includes(status)) return <LoaderCircle className="spin" aria-hidden="true"/>;
  return <CheckCircle2 aria-hidden="true"/>;
}

export default function PhotoSyncModule() {
  const [data,setData]=useState<Dashboard|null>(null);
  const [page,setPage]=useState(1);
  const [loading,setLoading]=useState(true);
  const [message,setMessage]=useState("");
  const [pairing,setPairing]=useState<Pairing|null>(null);
  const [qr,setQr]=useState("");
  const [remaining,setRemaining]=useState(0);
  const [selected,setSelected]=useState<Photo|null>(null);
  const [selectMode,setSelectMode]=useState(false);
  const [selectedIds,setSelectedIds]=useState<Set<string>>(new Set());
  const [vaultGate,setVaultGate]=useState<"create"|"unlock"|null>(null);
  const [vaultPassword,setVaultPassword]=useState("");
  const [vaultRepeat,setVaultRepeat]=useState("");
  const [vaultAccepted,setVaultAccepted]=useState(false);
  const [vaultBusy,setVaultBusy]=useState(false);
  const [vaultGateError,setVaultGateError]=useState("");
  const closeRef=useRef<HTMLButtonElement>(null);

  const load=useCallback(async(targetPage=page)=>{
    setLoading(true);
    try{
      const response=await fetch(`/api/photo-sync/dashboard?page=${targetPage}&pageSize=30`,{cache:"no-store"});
      const payload=await response.json() as Dashboard&{error?:string};
      if(!response.ok)throw new Error(payload.error||"照片数据读取失败");
      setData(payload);setPage(targetPage);
    }catch(error){setMessage(error instanceof Error?error.message:"照片数据读取失败")}
    finally{setLoading(false)}
  },[page]);

  useEffect(()=>{
    const timer=window.setTimeout(()=>void load(1),0);
    return()=>window.clearTimeout(timer);
  },[]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(()=>{
    if(!pairing)return;
    void QRCode.toDataURL(pairing.entryUrl,{width:240,margin:1,errorCorrectionLevel:"M"}).then(setQr);
    const update=()=>{
      const next=Math.max(0,Math.ceil((Date.parse(pairing.expiresAt)-Date.now())/1000));
      setRemaining(next);
      if(next===0)setPairing(current=>current?.pairCode===pairing.pairCode?null:current);
    };
    const initial=window.setTimeout(update,0);
    const timer=window.setInterval(update,1000);
    return()=>{window.clearTimeout(initial);window.clearInterval(timer)};
  },[pairing]);
  useEffect(()=>{
    if(!selected)return;
    closeRef.current?.focus();
    const close=(event:KeyboardEvent)=>{if(event.key==="Escape")setSelected(null)};
    window.addEventListener("keydown",close);return()=>window.removeEventListener("keydown",close);
  },[selected]);

  const groups=useMemo(()=>{
    const result=new Map<string,Photo[]>();
    for(const photo of data?.photos??[]){
      const day=new Intl.DateTimeFormat("zh-CN",{year:"numeric",month:"long",day:"numeric"})
        .format(new Date(photo.captured_at||photo.imported_at));
      result.set(day,[...(result.get(day)??[]),photo]);
    }
    return [...result.entries()];
  },[data?.photos]);

  const createPairing=async()=>{
    if(!window.photoSyncApi){setMessage("请在 LifeTrace Electron 桌面应用中创建配对二维码");return}
    setMessage("");
    const response=await window.photoSyncApi.createPairing();
    const status=response.status as {pairing?:Pairing}|undefined;
    if(!response.ok||!status?.pairing){setMessage(response.error||"无法创建配对码");return}
    setPairing(status.pairing);
  };
  const cancelPairing=async()=>{
    if(pairing&&window.photoSyncApi)await window.photoSyncApi.cancelPairing(pairing.pairCode);
    setPairing(null);setQr("");
  };
  const enterSelectMode=()=>{
    if(!data?.photos.length){setMessage("当前没有可隐藏的照片");return}
    setSelectMode(true);setSelectedIds(new Set());setMessage("");
  };
  const cancelSelect=()=>{setSelectMode(false);setSelectedIds(new Set())};
  const toggleSelect=(id:string)=>{
    setSelectedIds(current=>{
      const next=new Set(current);
      if(next.has(id))next.delete(id);else next.add(id);
      return next;
    });
  };
  const performHide=async(ids:string[])=>{
    setMessage("");
    try{
      const result=await window.vaultApi!.hidePhotosFromSyncAlbum(ids,null);
      if(!result.started){setMessage("隐藏任务未能启动，请重试。");return}
      // 立即从当前列表移除，加密在后台执行，做到无感隐藏。
      const idSet=new Set(ids);
      setData(current=>current?{
        ...current,
        photos:current.photos.filter(photo=>!idSet.has(photo.id)),
        total:Math.max(0,(current.total??0)-ids.length),
      }:current);
      setSelectMode(false);setSelectedIds(new Set());
      setMessage(`正在后台加密隐藏 ${result.count} 张照片，完成后可在私密相册查看。`);
    }catch(error){
      const text=error instanceof Error?error.message:String(error??"");
      setMessage(text||"隐藏任务启动失败");
    }
  };
  const hideSelected=async()=>{
    const ids=Array.from(selectedIds);
    if(!ids.length)return;
    if(typeof window.vaultApi?.hidePhotosFromSyncAlbum!=="function"){
      setMessage("当前版本缺少隐藏命令，请重新构建桌面端后重试。");
      return;
    }
    setMessage("");
    try{
      const vault=await window.vaultApi.status();
      if(!vault.configured){setVaultGate("create");return}
      if(!vault.unlocked){setVaultGate("unlock");return}
      await performHide(ids);
    }catch(error){
      const text=error instanceof Error?error.message:String(error??"");
      setMessage(text||"无法读取私密相册状态");
    }
  };
  const submitVaultGate=async()=>{
    if(!window.vaultApi||!vaultGate)return;
    if(vaultGate==="create"){
      if(!vaultAccepted){setVaultGateError("请先确认密码丢失后无法恢复");return}
      if(vaultPassword!==vaultRepeat){setVaultGateError("两次输入的密码不一致");return}
    }
    const gate=vaultGate;
    const password=vaultPassword;
    // 立即关闭弹窗，创建/解锁与隐藏全部在后台执行，界面不显示处理中状态。
    setVaultPassword("");setVaultRepeat("");setVaultAccepted(false);
    setVaultGate(null);
    setVaultBusy(true);setVaultGateError("");
    try{
      if(gate==="create"){
        await window.vaultApi.initialize(password);
      }else{
        await window.vaultApi.unlock(password);
      }
      await performHide(Array.from(selectedIds));
    }catch(error){
      const text=error instanceof Error?error.message:String(error??"");
      setMessage(text||"操作失败");
    }finally{
      setVaultBusy(false);
    }
  };
  const summary=data?.summary??{};
  const recentDevice=[...(data?.devices??[])].sort((a,b)=>Date.parse(b.last_seen_at||"0")-Date.parse(a.last_seen_at||"0"))[0];
  return <div className="hx-view photo-sync">
    <section className="photo-sync-hero">
      <div><span className="hx-pill">手机局域网</span><h2>把相册原文件，安静地收回本机</h2><p>同一局域网内用手机浏览器同步，不使用 iCloud 或 iOS 原生应用。</p></div>
      <button className="hx-btn primary" onClick={createPairing}><Smartphone/>添加 iPhone</button>
    </section>

    <section className="hx-metrics photo-sync-metrics" aria-label="最近同步概览">
      <div className="hx-metric"><span>最近同步</span><strong>{summary.last_sync_at?new Date(summary.last_sync_at).toLocaleDateString("zh-CN"):"暂无"}</strong><small>{formatDateTime(summary.last_sync_at)}</small></div>
      <div className="hx-metric"><span>最近设备</span><strong>{recentDevice?.device_name||"暂无"}</strong><small>{recentDevice?.status==="active"?"授权有效":"等待配对"}</small></div>
      <div className="hx-metric"><span>成功 / 重复</span><strong>{Number(summary.success_count||0)} / {Number(summary.duplicate_count||0)}</strong><small className="positive">重复原文件不会再次保存</small></div>
      <div className="hx-metric"><span>处理 / 失败</span><strong>{Number(summary.processing_count||0)} / {Number(summary.failed_count||0)}</strong><small>失败时仍保留原图记录</small></div>
    </section>

    <section className="photo-sync-layout">
      <div className="photo-timeline">
        <header className="photo-section-head"><div><span>照片时间线</span><h2>{data?.total??0} 个本地媒体文件</h2></div><div className="photo-section-actions">
          {selectMode
            ?<><span className="photo-select-count">已选 {selectedIds.size} 张</span><button className="hx-btn secondary" onClick={cancelSelect} disabled={loading}>取消</button><button className="hx-btn primary" onClick={()=>void hideSelected()} disabled={selectedIds.size===0||loading}><LockKeyhole/>隐藏到私密相册</button></>
            :<><button className="hx-btn secondary" onClick={enterSelectMode} disabled={loading}><EyeOff/>批量隐藏</button><button className="hx-btn secondary" onClick={()=>load(page)} disabled={loading}><RefreshCw className={loading?"spin":""}/>刷新</button></>}
        </div></header>
        {selectMode&&<p className="photo-select-hint">选择要隐藏的照片，确认后会被加密移入私密相册并从同步相册移除。</p>}
        {loading&&!data?<div className="photo-loading"><LoaderCircle className="spin"/><p>正在读取缩略图索引…</p></div>:
          groups.length?groups.map(([day,photos])=><section className="photo-day" key={day}>
            <header><h3>{day}</h3><span>{photos.length} 项</span></header>
            <div className="photo-grid">{photos.map(photo=><button className={`photo-card${selectMode?" selectable":""}${selectedIds.has(photo.id)?" selected":""}`} key={photo.id} onClick={()=>selectMode?toggleSelect(photo.id):setSelected(photo)} aria-label={selectMode?`选择 ${photo.original_file_name}`:`查看 ${photo.original_file_name}`}>
              {selectedIds.has(photo.id)&&<i className="photo-select-mark">✓</i>}
              <span className="photo-thumb">
                {photo.processing_status==="completed"
                  ?<img src={`${mediaBase}/${photo.id}/thumbnail`} alt="" loading="lazy" decoding="async"/>
                  :<span className={`photo-placeholder ${photo.processing_status}`}><StateIcon status={photo.processing_status}/><small>{statusLabel[photo.processing_status]||photo.processing_status}</small></span>}
                {photo.media_type==="video"&&<i><Film/>视频</i>}
              </span>
              <span className="photo-card-copy"><strong>{new Date(photo.captured_at||photo.imported_at).toLocaleTimeString("zh-CN",{hour:"2-digit",minute:"2-digit"})}</strong><small>{photo.device_name||"iPhone"} · {formatBytes(photo.file_size)}</small></span>
            </button>)}</div>
          </section>):<div className="photo-empty"><ImageIcon/><h3>还没有同步照片</h3><p>添加手机后，通过浏览器上传成功的照片会按拍摄日期出现在这里。</p></div>}
        {(data?.total??0)>30&&<footer className="photo-pagination">
          <button className="hx-btn secondary" disabled={page<=1||loading} onClick={()=>load(page-1)}>上一页</button>
          <span>第 {page} / {Math.ceil((data?.total??0)/30)} 页</span>
          <button className="hx-btn secondary" disabled={page>=Math.ceil((data?.total??0)/30)||loading} onClick={()=>load(page+1)}>下一页</button>
        </footer>}
      </div>
    </section>

    {message&&<Toast message={message} onClose={()=>setMessage("")}/>}

    {vaultGate&&<div className="hx-overlay photo-vault-gate" role="dialog" aria-modal="true" aria-labelledby="vault-gate-title" onMouseDown={event=>{if(event.target===event.currentTarget&&!vaultBusy)setVaultGate(null)}}>
      <article className="photo-vault-gate-card"><header><div><span>私密相册</span><h2 id="vault-gate-title">{vaultGate==="create"?"创建私密相册":"解锁私密相册"}</h2><p>{vaultGate==="create"?"隐藏的照片会加密保存在本机，密码丢失后任何人都无法恢复。":"输入密码解锁后即可把所选照片隐藏进去。"}</p></div><button aria-label="关闭" disabled={vaultBusy} onClick={()=>setVaultGate(null)}><X/></button></header>
        <div className="photo-vault-gate-body">
          {vaultGate==="create"?<>
            <label>私密相册密码<input type="password" minLength={6} value={vaultPassword} onChange={event=>setVaultPassword(event.target.value)} autoComplete="new-password"/></label>
            <label>再次输入密码<input type="password" minLength={6} value={vaultRepeat} onChange={event=>setVaultRepeat(event.target.value)} autoComplete="new-password"/></label>
            <label className="photo-vault-confirm"><input type="checkbox" checked={vaultAccepted} onChange={event=>setVaultAccepted(event.target.checked)}/><span>我确认密码丢失后，任何人都无法恢复其中内容。</span></label>
          </>:<label>密码<input type="password" value={vaultPassword} onChange={event=>setVaultPassword(event.target.value)} autoFocus autoComplete="current-password"/></label>}
          {vaultGateError&&<p className="photo-vault-gate-error" role="alert">{vaultGateError}</p>}
          <button className="hx-btn primary" disabled={vaultBusy||!vaultPassword||(vaultGate==="create"&&(!vaultAccepted||vaultPassword.length<6||vaultPassword!==vaultRepeat))} onClick={()=>void submitVaultGate()}>{vaultGate==="create"?"创建并隐藏所选照片":"解锁并隐藏所选照片"}</button>
        </div>
      </article>
    </div>}

    {pairing&&<div className="hx-overlay photo-pair-overlay" role="dialog" aria-modal="true" aria-labelledby="pair-title" onMouseDown={event=>{if(event.target===event.currentTarget)void cancelPairing()}}>
      <article className="photo-pair-modal"><header><div><span>一次性配对</span><h2 id="pair-title">扫描二维码添加 iPhone</h2></div><button aria-label="取消配对" onClick={cancelPairing}><X/></button></header>
        <div className="photo-pair-body">{qr?<img src={qr} alt="打开 LifeTrace 照片同步配对页面的二维码"/>:<LoaderCircle className="spin"/>}
          <div><span>配对码</span><strong>{pairing.pairCode}</strong><button onClick={()=>navigator.clipboard.writeText(pairing.pairCode)}><Copy/>复制</button></div>
          <p>二维码只包含一次性配对码和局域网地址，不包含长期设备令牌。</p>
          <b className={remaining<60?"urgent":""}>{remaining>0?`${Math.floor(remaining/60)}:${String(remaining%60).padStart(2,"0")} 后失效`:"配对码已失效"}</b>
        </div>
      </article>
    </div>}

    {selected&&<div className="hx-overlay photo-preview" role="dialog" aria-modal="true" aria-label={selected.original_file_name} onMouseDown={event=>{if(event.target===event.currentTarget)setSelected(null)}}>
      <article><header><div><strong>{selected.original_file_name}</strong><small>{formatDateTime(selected.captured_at)} · {selected.device_name||"iPhone"} · {formatBytes(selected.file_size)}</small></div><button ref={closeRef} aria-label="关闭预览" onClick={()=>setSelected(null)}><X/></button></header>
        <div className="photo-preview-media">{selected.processing_status==="completed"
          ?selected.media_type==="video"
            ?<video controls preload="metadata" poster={`${mediaBase}/${selected.id}/thumbnail`} src={`${mediaBase}/${selected.id}/original`}/>
            :<img src={`${mediaBase}/${selected.id}/original`} alt={selected.original_file_name}/>
          :<div className="photo-preview-error"><StateIcon status={selected.processing_status}/><h3>{statusLabel[selected.processing_status]||selected.processing_status}</h3><p>{selected.processing_error||"原文件已保存，缩略图仍在处理中。"}</p></div>}</div>
        <footer><span>{selected.width&&selected.height?`${selected.width} × ${selected.height}`:"尺寸待提取"}{selected.duration_ms?` · ${Math.round(selected.duration_ms/1000)} 秒`:""}</span><span>同步于 {formatDateTime(selected.imported_at)}</span></footer>
      </article>
    </div>}
  </div>;
}
