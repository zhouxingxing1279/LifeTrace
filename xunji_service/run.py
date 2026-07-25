"""本地启动：python xunji_service/run.py"""

import uvicorn


if __name__ == "__main__":
    uvicorn.run("app.main:app", host="127.0.0.1", port=8001, reload=False, app_dir="xunji_service")

