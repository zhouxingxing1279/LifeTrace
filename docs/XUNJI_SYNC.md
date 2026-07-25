# 训记训练数据同步

## 本地结构

- 网页端只调用同源的 `/api/xunji/*`。
- FastAPI 监听 `127.0.0.1:8001`，负责二维码识别和训记网页解析。
- D1/SQLite 负责待确认记录、训练历史、习惯打卡与训练日志。
- 分享图片只在请求内存中读取，不保存，也不进行 OCR。

## 启动

首次安装：

```powershell
python -m venv .venv-xunji
.\.venv-xunji\Scripts\python.exe -m pip install -r xunji_service\requirements.txt
.\.venv-xunji\Scripts\python.exe -m playwright install chromium
```

启动解析服务：

```powershell
.\.venv-xunji\Scripts\python.exe xunji_service\run.py
```

网页服务默认通过 `http://127.0.0.1:8001` 调用它，也可以用
`XUNJI_SERVICE_URL` 覆盖。

## 安全边界

- 二维码和全部重定向只能指向 `https://api.xunjiapp.cn/app_share/*`。
- Playwright 会拦截并阻止其他域名的请求。
- 图片上限 15MB，网页响应上限 6MB。
- 网页没有结构化训练数据时，调试文件保存在
  `xunji_service/debug/<时间>/`，包含 `page.html`、`network.json`
  和 `response.json`。

## 解析顺序

1. `window.Train.movement`、`__NEXT_DATA__`、`window.__INITIAL_STATE__` 和 JSON script。
2. Playwright 动态页面的 JSON 网络响应。
3. HTML DOM 中的训练动作与组数。

确认导入前，记录状态为 `pending`；只有用户点击“确认导入”才会
创建 `workout_history`、健身习惯打卡和 `training_notes`。
