# LifeTrace 离线词典

`dictionary.db` 是应用运行时读取的只读 SQLite 词典。仓库内附带的小型 starter 数据只用于让首次运行和测试具备基础离线查词能力，不替代完整 ECDICT。

完整词典请从 ECDICT 官方仓库取得 `ecdict.csv`（或 `stardict.csv`）和 `lemma.en.txt` 后执行：

```powershell
.\.venv-xunji\Scripts\python.exe scripts\import_ecdict.py D:\path\ecdict.csv `
  --lemma D:\path\lemma.en.txt `
  --output xunji_service\data\dictionary.db `
  --rebuild
```

应用启动时不会导入、下载或更新词典。
