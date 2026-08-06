"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle, FileVideo, Image as ImageIcon, KeyRound, LoaderCircle,
  Lock, LockKeyhole, ShieldCheck, Trash2, Upload, X,
} from "lucide-react";

const DELETE_CONFIRMATION = "永久删除私密相册";

function readableSize(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 ** 2) return `${(value / 1024).toFixed(1)} KB`;
  if (value < 1024 ** 3) return `${(value / 1024 ** 2).toFixed(1)} MB`;
  return `${(value / 1024 ** 3).toFixed(1)} GB`;
}

function dataUrl(mimeType: string, dataBase64: string) {
  return `data:${mimeType};base64,${dataBase64}`;
}

function passwordIssue(password: string) {
  if ([...password].length < 12) return "密码至少需要 12 个字符";
  if (/^\d+$/.test(password)) return "密码不能只包含数字";
  return "";
}

export default function LocalVaultModule() {
  const api = window.vaultApi;
  const [status, setStatus] = useState<VaultStatus | null>(null);
  const [assets, setAssets] = useState<VaultAsset[]>([]);
  const [thumbnails, setThumbnails] = useState<Record<string, string>>({});
  const [password, setPassword] = useState("");
  const [passwordAgain, setPasswordAgain] = useState("");
  const [oldPassword, setOldPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [newPasswordAgain, setNewPasswordAgain] = useState("");
  const [deleteConfirmation, setDeleteConfirmation] = useState("");
  const [showDangerZone, setShowDangerZone] = useState(false);
  const [showPasswordChange, setShowPasswordChange] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [selected, setSelected] = useState<{ asset: VaultAsset; url: string } | null>(null);
  const passwordInput = useRef<HTMLInputElement>(null);

  const clearDecryptedState = useCallback(() => {
    setAssets([]);
    setThumbnails({});
    setSelected(null);
  }, []);

  const loadAssets = useCallback(async () => {
    if (!api) return;
    const nextAssets = await api.listAssets();
    setAssets(nextAssets);
    const nextThumbnails: Record<string, string> = {};
    await Promise.all(nextAssets.filter((asset) => asset.hasThumbnail).map(async (asset) => {
      try {
        const thumbnail = await api.readThumbnail(asset.id);
        nextThumbnails[asset.id] = dataUrl(thumbnail.mimeType, thumbnail.dataBase64);
      } catch {
        // A missing/corrupted thumbnail must not prevent access to the encrypted original.
      }
    }));
    setThumbnails(nextThumbnails);
  }, [api]);

  const refresh = useCallback(async () => {
    if (!api) {
      setError("当前运行环境不支持本地私密相册");
      return;
    }
    try {
      const nextStatus = await api.status();
      setStatus(nextStatus);
      if (nextStatus.unlocked) await loadAssets();
      else clearDecryptedState();
    } catch (reason) {
      setError(String(reason));
    }
  }, [api, clearDecryptedState, loadAssets]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (status?.configured && !status.unlocked) passwordInput.current?.focus();
  }, [status?.configured, status?.unlocked]);

  const run = useCallback(async (operation: () => Promise<void>) => {
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await operation();
    } catch (reason) {
      setError(String(reason).replace(/^Error:\s*/, ""));
    } finally {
      setBusy(false);
    }
  }, []);

  const initialize = () => void run(async () => {
    if (!api) return;
    const issue = passwordIssue(password);
    if (issue) throw new Error(issue);
    if (password !== passwordAgain) throw new Error("两次输入的密码不一致");
    const nextStatus = await api.initialize(password);
    setStatus(nextStatus);
    setPassword("");
    setPasswordAgain("");
    setMessage("私密相册已创建。请牢记密码，系统无法找回。 ");
    await loadAssets();
  });

  const unlock = () => void run(async () => {
    if (!api) return;
    const nextStatus = await api.unlock(password);
    setStatus(nextStatus);
    setPassword("");
    await loadAssets();
  });

  const lock = () => void run(async () => {
    if (!api) return;
    const nextStatus = await api.lock();
    clearDecryptedState();
    setStatus(nextStatus);
    setMessage("私密相册已锁定，解密内容已从界面清除。");
  });

  const importFiles = () => void run(async () => {
    if (!api) return;
    const imported = await api.importFiles();
    if (!imported.length) return;
    const nextStatus = await api.status();
    setStatus(nextStatus);
    await loadAssets();
    setMessage(`已在本机加密导入 ${imported.length} 个文件。原文件不会上传到任何云端。`);
  });

  const openAsset = (asset: VaultAsset) => void run(async () => {
    if (!api) return;
    const payload = await api.readAsset(asset.id);
    setSelected({ asset: payload.asset, url: dataUrl(payload.asset.mimeType, payload.dataBase64) });
  });

  const deleteAsset = (asset: VaultAsset) => void run(async () => {
    if (!api) return;
    if (!window.confirm(`确定永久删除“${asset.originalName}”吗？此操作不可恢复。`)) return;
    await api.deleteAsset(asset.id);
    if (selected?.asset.id === asset.id) setSelected(null);
    await loadAssets();
    const nextStatus = await api.status();
    setStatus(nextStatus);
  });

  const changePassword = () => void run(async () => {
    if (!api) return;
    const issue = passwordIssue(newPassword);
    if (issue) throw new Error(issue);
    if (newPassword !== newPasswordAgain) throw new Error("两次输入的新密码不一致");
    const nextStatus = await api.changePassword(oldPassword, newPassword);
    setStatus(nextStatus);
    setOldPassword("");
    setNewPassword("");
    setNewPasswordAgain("");
    setShowPasswordChange(false);
    setMessage("私密相册密码已修改。旧密码已失效。 ");
  });

  const deleteVault = () => void run(async () => {
    if (!api) return;
    if (deleteConfirmation !== DELETE_CONFIRMATION) throw new Error("请输入完整的删除确认文本");
    await api.deleteAll(deleteConfirmation);
    clearDecryptedState();
    setDeleteConfirmation("");
    setShowDangerZone(false);
    setStatus(await api.status());
    setMessage("私密相册及其本地密文已永久删除。");
  });

  const setAutoLock = (seconds: number) => void run(async () => {
    if (!api) return;
    setStatus(await api.setAutoLock(seconds));
    setMessage("自动锁定时间已更新。");
  });

  const photoCount = useMemo(() => assets.filter((asset) => asset.mimeType.startsWith("image/")).length, [assets]);
  const videoCount = useMemo(() => assets.filter((asset) => asset.mimeType.startsWith("video/")).length, [assets]);

  if (!api) {
    return <section className="local-vault unavailable"><AlertTriangle/><h2>私密相册不可用</h2><p>该功能只在 LifeTrace 桌面端本地运行。</p></section>;
  }

  if (!status) {
    return <section className="local-vault loading"><LoaderCircle className="spin"/><p>正在读取本地私密相册状态…</p></section>;
  }

  if (!status.configured) {
    return <section className="local-vault vault-gate">
      <div className="vault-gate-card">
        <span className="vault-gate-icon"><LockKeyhole/></span>
        <p className="vault-kicker">仅本机 · 零恢复</p>
        <h2>创建私密相册</h2>
        <p>照片、视频、缩略图和元数据只会保存在当前电脑，并使用你的密码派生密钥进行加密。</p>
        <div className="vault-warning"><AlertTriangle/><span>不提供密码找回、恢复密钥或管理员重置。忘记密码后，全部内容永久无法恢复。</span></div>
        <label>设置独立密码<input type="password" autoComplete="new-password" value={password} onChange={(event)=>setPassword(event.target.value)} placeholder="至少 12 个字符，不能是纯数字"/></label>
        <label>再次输入密码<input type="password" autoComplete="new-password" value={passwordAgain} onChange={(event)=>setPasswordAgain(event.target.value)} onKeyDown={(event)=>{if(event.key==="Enter")initialize()}}/></label>
        {error && <p className="vault-error">{error}</p>}
        <button className="vault-primary" disabled={busy} onClick={initialize}>{busy?<LoaderCircle className="spin"/>:<ShieldCheck/>}创建本地加密相册</button>
      </div>
    </section>;
  }

  if (!status.unlocked) {
    return <section className="local-vault vault-gate">
      <div className="vault-gate-card">
        <span className="vault-gate-icon"><Lock/></span>
        <p className="vault-kicker">私密相册已锁定</p>
        <h2>输入密码解锁</h2>
        <p>锁定状态下不会显示照片数量、封面、文件名或访问记录。</p>
        <label>私密相册密码<input ref={passwordInput} type="password" autoComplete="current-password" value={password} onChange={(event)=>setPassword(event.target.value)} onKeyDown={(event)=>{if(event.key==="Enter")unlock()}}/></label>
        {error && <p className="vault-error">{error}</p>}
        {message && <p className="vault-message">{message}</p>}
        <button className="vault-primary" disabled={busy||!password} onClick={unlock}>{busy?<LoaderCircle className="spin"/>:<KeyRound/>}解锁</button>
        <button className="vault-danger-link" onClick={()=>setShowDangerZone((value)=>!value)}>忘记密码，只能永久删除</button>
        {showDangerZone && <div className="vault-danger-zone">
          <strong>永久删除私密相册</strong>
          <p>无需密码即可删除，但所有密文、缩略图和加密清单都会永久消失。</p>
          <label>输入“{DELETE_CONFIRMATION}”<input value={deleteConfirmation} onChange={(event)=>setDeleteConfirmation(event.target.value)}/></label>
          <button className="vault-danger" disabled={busy||deleteConfirmation!==DELETE_CONFIRMATION} onClick={deleteVault}><Trash2/>永久删除</button>
        </div>}
      </div>
    </section>;
  }

  return <section className="local-vault vault-open">
    <header className="vault-header">
      <div><p className="vault-kicker">仅本机加密存储</p><h2><ShieldCheck/>私密相册</h2><p>所有内容均由 Rust 本地进程加解密，不进入照片同步、云端备份或远程 AI。</p></div>
      <div className="vault-actions">
        <button onClick={importFiles} disabled={busy}><Upload/>导入本地文件</button>
        <button onClick={()=>setShowPasswordChange((value)=>!value)}><KeyRound/>修改密码</button>
        <button className="vault-lock" onClick={lock}><Lock/>锁定</button>
      </div>
    </header>

    {(error||message) && <div className={error?"vault-notice error":"vault-notice"}>{error||message}<button onClick={()=>{setError("");setMessage("")}}><X/></button></div>}

    <div className="vault-summary">
      <div><strong>{status.assetCount??assets.length}</strong><span>加密文件</span></div>
      <div><strong>{photoCount}</strong><span>照片</span></div>
      <div><strong>{videoCount}</strong><span>视频</span></div>
      <label>自动锁定<select value={status.autoLockSeconds} onChange={(event)=>setAutoLock(Number(event.target.value))}>
        <option value={30}>30 秒</option><option value={60}>1 分钟</option><option value={300}>5 分钟</option><option value={600}>10 分钟</option><option value={1800}>30 分钟</option>
      </select></label>
    </div>

    {showPasswordChange && <div className="vault-inline-form">
      <h3>修改密码</h3><p>必须验证当前密码。没有当前密码无法保留数据重置。</p>
      <input type="password" autoComplete="current-password" value={oldPassword} onChange={(event)=>setOldPassword(event.target.value)} placeholder="当前密码"/>
      <input type="password" autoComplete="new-password" value={newPassword} onChange={(event)=>setNewPassword(event.target.value)} placeholder="新密码（至少 12 个字符）"/>
      <input type="password" autoComplete="new-password" value={newPasswordAgain} onChange={(event)=>setNewPasswordAgain(event.target.value)} placeholder="再次输入新密码"/>
      <div><button onClick={()=>setShowPasswordChange(false)}>取消</button><button className="vault-primary" onClick={changePassword} disabled={busy}>确认修改</button></div>
    </div>}

    {busy && <div className="vault-progress"><LoaderCircle className="spin"/>正在执行本地加密操作…</div>}

    {!assets.length&&!busy ? <div className="vault-empty"><LockKeyhole/><h3>私密相册为空</h3><p>选择本机照片或视频后，文件会先加密再写入私密目录。</p><button className="vault-primary" onClick={importFiles}><Upload/>导入文件</button></div> : <div className="vault-grid">
      {assets.map((asset)=><article key={asset.id} className="vault-card">
        <button className="vault-card-preview" onClick={()=>openAsset(asset)} aria-label={`查看 ${asset.originalName}`}>
          {thumbnails[asset.id]?<img src={thumbnails[asset.id]} alt=""/>:asset.mimeType.startsWith("video/")?<FileVideo/>:<ImageIcon/>}
          <span><Lock/>本地密文</span>
        </button>
        <div><strong title={asset.originalName}>{asset.originalName}</strong><small>{readableSize(asset.size)} · {new Date(asset.importedAt).toLocaleString("zh-CN",{hour12:false})}</small></div>
        <button className="vault-card-delete" onClick={()=>deleteAsset(asset)} aria-label={`删除 ${asset.originalName}`}><Trash2/></button>
      </article>)}
    </div>}

    <footer className="vault-footer"><ShieldCheck/><span>私密数据不会上传、同步、分享或发送给远程 AI 服务。</span><button className="vault-danger-link" onClick={()=>setShowDangerZone((value)=>!value)}>删除整个私密相册</button></footer>
    {showDangerZone && <div className="vault-danger-zone open-danger">
      <strong>永久删除整个私密相册</strong><p>此操作不需要密码，但不可撤销。输入“{DELETE_CONFIRMATION}”确认。</p>
      <input value={deleteConfirmation} onChange={(event)=>setDeleteConfirmation(event.target.value)}/>
      <button className="vault-danger" disabled={busy||deleteConfirmation!==DELETE_CONFIRMATION} onClick={deleteVault}><Trash2/>永久删除全部内容</button>
    </div>}

    {selected && <div className="vault-modal" role="dialog" aria-modal="true" aria-label={selected.asset.originalName} onMouseDown={(event)=>{if(event.target===event.currentTarget)setSelected(null)}}>
      <div><header><strong>{selected.asset.originalName}</strong><button onClick={()=>setSelected(null)}><X/></button></header>
        {selected.asset.mimeType.startsWith("video/")?<video src={selected.url} controls autoPlay/>:<img src={selected.url} alt={selected.asset.originalName}/>} 
      </div>
    </div>}
  </section>;
}
