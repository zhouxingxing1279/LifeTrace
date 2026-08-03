"use client";
/* eslint-disable @next/next/no-img-element -- local authenticated media URLs are not compatible with the image optimizer */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import QRCode from "qrcode";
import {
  AlertTriangle, CheckCircle2, Clock3, Copy, Download, Film, Image as ImageIcon,
  LoaderCircle, MonitorUp, RefreshCw, ShieldCheck, Smartphone, Unplug, X,
} from "lucide-react";

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
  const [serverStatus,setServerStatus]=useState<MobileUploadStatus|null>(null);
  const closeRef=useRef<HTMLButtonElement>(null);

  const load=useCallback(async(targetPage=page)=>{
    setLoading(true);
    try{
      const response=await fetch(`/api/photo-sync/dashboard?page=${targetPage}&pageSize=30`,{cache:"no-store"});
      const payload=await response.json() as Dashboard&{error?:string};
      if(!response.ok)throw new Error(payload.error||"照片数据读取失败");
      setData(payload);setPage(targetPage);
      if(window.photoSyncApi){
        const status=await window.photoSyncApi.status();
        if(status.ok&&status.status)setServerStatus(status.status);
      }
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
    const status=response.status as (MobileUploadStatus&{pairing?:Pairing})|undefined;
    if(!response.ok||!status?.pairing){setMessage(response.error||"无法创建配对码");return}
    setServerStatus(status);setPairing(status.pairing);
  };
  const cancelPairing=async()=>{
    if(pairing&&window.photoSyncApi)await window.photoSyncApi.cancelPairing(pairing.pairCode);
    setPairing(null);setQr("");
  };
  const revoke=async(device:Device)=>{
    if(!window.confirm(`撤销“${device.device_name}”的照片同步授权？撤销后该设备会立即无法继续上传。`))return;
    const response=await fetch("/api/photo-sync/dashboard",{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify({action:"revokeDevice",deviceId:device.id})});
    if(!response.ok){setMessage("设备授权撤销失败");return}
    await load(page);
  };
  const retry=async(task:UploadTask)=>{
    if(!task.photo_id)return;
    const response=await fetch("/api/photo-sync/dashboard",{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify({action:"retryProcessing",photoId:task.photo_id})});
    if(!response.ok){setMessage("重新处理任务失败");return}
    await window.photoSyncApi?.recover();
    await load(page);
  };
  const exportCertificate=async()=>{
    if(!window.photoSyncApi){setMessage("请在 LifeTrace 桌面应用中导出 iPhone 信任证书");return}
    setMessage("");
    const response=await window.photoSyncApi.exportCertificate();
    if(!response.ok){setMessage(response.error||"证书导出失败");return}
    if(response.status?.certificateExported){
      setServerStatus(response.status);
      setMessage(`证书已保存：${response.status.certificateExportPath||"请在保存位置查看"}`);
    }
  };
  const setCompatibilityMode=async(enabled:boolean)=>{
    if(!window.photoSyncApi){setMessage("请在 LifeTrace 桌面应用中切换传输模式");return}
    if(enabled&&!window.confirm("开启后，设备令牌和照片会在当前局域网内以明文传输，可能被同一网络中的其他设备监听。仅应在可信的家庭网络中临时使用。确定开启吗？"))return;
    setMessage("");
    const response=await window.photoSyncApi.setCompatibilityMode(enabled,enabled);
    if(!response.ok||!response.status){setMessage(response.error||"传输模式切换失败");return}
    setServerStatus(response.status);
    setPairing(null);setQr("");
    setMessage(enabled
      ?"HTTP 兼容模式已开启。请重新点击“添加 iPhone”生成二维码。"
      :"HTTPS 安全模式已恢复。已有 HTTP 二维码已失效，请重新生成。");
  };

  const summary=data?.summary??{};
  const recentDevice=[...(data?.devices??[])].sort((a,b)=>Date.parse(b.last_seen_at||"0")-Date.parse(a.last_seen_at||"0"))[0];
  return <div className="hx-view photo-sync">
    <section className="photo-sync-hero">
      <div><span className="hx-pill">iPhone 快捷指令</span><h2>把相册原文件，安静地收回本机</h2><p>同一局域网内增量同步，不使用 iCloud、PWA 或 iOS 原生应用。</p></div>
      <button className="hx-btn primary" onClick={createPairing}><Smartphone/>添加 iPhone</button>
    </section>

    {message&&<div className="photo-sync-message" role="alert"><AlertTriangle/>{message}<button aria-label="关闭提示" onClick={()=>setMessage("")}><X/></button></div>}

    <section className="hx-metrics photo-sync-metrics" aria-label="最近同步概览">
      <div className="hx-metric"><span>最近同步</span><strong>{summary.last_sync_at?new Date(summary.last_sync_at).toLocaleDateString("zh-CN"):"暂无"}</strong><small>{formatDateTime(summary.last_sync_at)}</small></div>
      <div className="hx-metric"><span>最近设备</span><strong>{recentDevice?.device_name||"暂无"}</strong><small>{recentDevice?.status==="active"?"授权有效":"等待配对"}</small></div>
      <div className="hx-metric"><span>成功 / 重复</span><strong>{Number(summary.success_count||0)} / {Number(summary.duplicate_count||0)}</strong><small className="positive">重复原文件不会再次保存</small></div>
      <div className="hx-metric"><span>处理 / 失败</span><strong>{Number(summary.processing_count||0)} / {Number(summary.failed_count||0)}</strong><small>失败时仍保留原图记录</small></div>
    </section>

    <section className="photo-sync-layout">
      <div className="photo-timeline">
        <header className="photo-section-head"><div><span>照片时间线</span><h2>{data?.total??0} 个本地媒体文件</h2></div><button className="hx-btn secondary" onClick={()=>load(page)} disabled={loading}><RefreshCw className={loading?"spin":""}/>刷新</button></header>
        {loading&&!data?<div className="photo-loading"><LoaderCircle className="spin"/><p>正在读取缩略图索引…</p></div>:
          groups.length?groups.map(([day,photos])=><section className="photo-day" key={day}>
            <header><h3>{day}</h3><span>{photos.length} 项</span></header>
            <div className="photo-grid">{photos.map(photo=><button className="photo-card" key={photo.id} onClick={()=>setSelected(photo)} aria-label={`查看 ${photo.original_file_name}`}>
              <span className="photo-thumb">
                {photo.processing_status==="completed"
                  ?<img src={`${mediaBase}/${photo.id}/thumbnail`} alt="" loading="lazy" decoding="async"/>
                  :<span className={`photo-placeholder ${photo.processing_status}`}><StateIcon status={photo.processing_status}/><small>{statusLabel[photo.processing_status]||photo.processing_status}</small></span>}
                {photo.media_type==="video"&&<i><Film/>视频</i>}
              </span>
              <span className="photo-card-copy"><strong>{new Date(photo.captured_at||photo.imported_at).toLocaleTimeString("zh-CN",{hour:"2-digit",minute:"2-digit"})}</strong><small>{photo.device_name||"iPhone"} · {formatBytes(photo.file_size)}</small></span>
            </button>)}</div>
          </section>):<div className="photo-empty"><ImageIcon/><h3>还没有同步照片</h3><p>添加 iPhone 后，快捷指令上传成功的照片会按拍摄日期出现在这里。</p></div>}
        {(data?.total??0)>30&&<footer className="photo-pagination">
          <button className="hx-btn secondary" disabled={page<=1||loading} onClick={()=>load(page-1)}>上一页</button>
          <span>第 {page} / {Math.ceil((data?.total??0)/30)} 页</span>
          <button className="hx-btn secondary" disabled={page>=Math.ceil((data?.total??0)/30)||loading} onClick={()=>load(page+1)}>下一页</button>
        </footer>}
      </div>

      <aside className="photo-sync-side">
        <article className="hx-panel"><header className="photo-side-head"><div><span>局域网服务</span><h3>{serverStatus?.active?"正在运行":"等待开启"}</h3></div>{serverStatus?.active?<ShieldCheck/>:<Unplug/>}</header>
          <div className="photo-info-list"><span>电脑名称<b>{serverStatus?.computerName||"LifeTrace-PC"}</b></span><span>监听地址<b>{serverStatus?.bindAddress||"0.0.0.0"}:{serverStatus?.port||3443}</b></span><span>手机地址<b>{serverStatus?.photoSyncUrls?.[0]||"添加设备时自动显示"}</b></span></div>
          {serverStatus?.allowInsecureHttp
            ?<div className="photo-transport-warning" role="status"><AlertTriangle aria-hidden="true"/><div><strong>HTTP 兼容模式已开启</strong><span>照片与设备令牌未加密，仅限可信局域网临时使用。</span></div></div>
            :<><button className="photo-certificate-action" onClick={exportCertificate}><Download aria-hidden="true"/>导出 iPhone 信任证书</button>
              <p className="photo-certificate-note">首次使用时安装并完全信任一次。电脑 IP 变化后，LifeTrace 会自动更新服务器证书，无需重新安装。</p></>}
          <button className={`photo-compatibility-action${serverStatus?.allowInsecureHttp?" active":""}`} onClick={()=>setCompatibilityMode(!serverStatus?.allowInsecureHttp)}>
            {serverStatus?.allowInsecureHttp?"关闭兼容模式，恢复 HTTPS":"快捷指令证书无效？开启 HTTP 兼容模式"}
          </button>
          <p className="photo-hint">仅开放同一局域网；Windows 首次提示时允许“专用网络”。权限最终由设备 Token 判断，不只依赖 IP。</p>
        </article>

        <article className="hx-panel"><header className="photo-side-head"><div><span>同步设备</span><h3>{data?.devices.length??0} 台</h3></div><Smartphone/></header>
          <div className="photo-device-list">{data?.devices.length?data.devices.map(device=><div key={device.id}><span><Smartphone/><i className={device.status}/></span><div><strong>{device.device_name}</strong><small>配对 {formatDateTime(device.paired_at)}<br/>在线 {formatDateTime(device.last_seen_at)}</small></div><button disabled={device.status==="revoked"} onClick={()=>revoke(device)}>{device.status==="active"?"撤销":"已撤销"}</button></div>):<p className="photo-side-empty">尚未配对 iPhone</p>}</div>
        </article>

        <article className="hx-panel"><header className="photo-side-head"><div><span>上传与处理</span><h3>{data?.tasks.length??0} 个待处理任务</h3></div><MonitorUp/></header>
          <div className="photo-task-list">{data?.tasks.length?data.tasks.map(task=><div key={task.id}><StateIcon status={task.status}/><div><strong>{task.original_file_name}</strong><small>{statusLabel[task.status]||task.status} · {formatBytes(task.received_file_size||task.expected_file_size)}{task.error_code?` · ${task.error_code}`:""}</small>{task.error_message&&<p>{task.error_message}</p>}</div>{task.status==="failed"&&task.photo_id&&<button onClick={()=>retry(task)}>重试</button>}</div>):<p className="photo-side-empty">没有失败或进行中的任务</p>}</div>
          <button className="photo-cleanup" onClick={async()=>{await window.photoSyncApi?.recover();await load(page)}}><Clock3/>清理过期临时文件并恢复任务</button>
        </article>
      </aside>
    </section>

    {pairing&&<div className="hx-overlay photo-pair-overlay" role="dialog" aria-modal="true" aria-labelledby="pair-title" onMouseDown={event=>{if(event.target===event.currentTarget)void cancelPairing()}}>
      <article className="photo-pair-modal"><header><div><span>一次性配对</span><h2 id="pair-title">扫描二维码添加 iPhone</h2></div><button aria-label="取消配对" onClick={cancelPairing}><X/></button></header>
        <div className="photo-pair-body">{qr?<img src={qr} alt="打开 LifeTrace 照片同步快捷指令的配对二维码"/>:<LoaderCircle className="spin"/>}
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
