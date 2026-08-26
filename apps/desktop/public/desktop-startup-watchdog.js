(function () {
  var FAILURE_DELAY_MS = 15000;
  var failureShown = false;

  function rootElement() {
    return document.getElementById("root");
  }

  function showFailure(title, detail) {
    if (failureShown || window.__LIFETRACE_MODULE_STARTED__) return;
    var root = rootElement();
    if (!root) return;
    failureShown = true;
    root.setAttribute("data-lifetrace-boot-pending", "false");
    root.innerHTML = "";

    var panel = document.createElement("div");
    panel.setAttribute("role", "alert");
    panel.style.minHeight = "100vh";
    panel.style.display = "grid";
    panel.style.placeContent = "center";
    panel.style.justifyItems = "center";
    panel.style.gap = "12px";
    panel.style.padding = "32px";
    panel.style.boxSizing = "border-box";
    panel.style.background = "#f4f5f4";
    panel.style.color = "#17211e";
    panel.style.fontFamily = "system-ui,-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif";
    panel.style.textAlign = "center";

    var badge = document.createElement("div");
    badge.textContent = "!";
    badge.style.display = "grid";
    badge.style.width = "52px";
    badge.style.height = "52px";
    badge.style.placeItems = "center";
    badge.style.borderRadius = "16px";
    badge.style.background = "#17211e";
    badge.style.color = "#fff";
    badge.style.fontWeight = "800";

    var heading = document.createElement("strong");
    heading.textContent = title;
    heading.style.fontSize = "18px";

    var message = document.createElement("span");
    message.textContent = detail;
    message.style.maxWidth = "720px";
    message.style.fontSize = "13px";
    message.style.lineHeight = "1.6";
    message.style.color = "#68756f";

    var retry = document.createElement("button");
    retry.textContent = "重新启动";
    retry.style.marginTop = "4px";
    retry.style.padding = "9px 16px";
    retry.style.border = "0";
    retry.style.borderRadius = "10px";
    retry.style.background = "#17211e";
    retry.style.color = "#fff";
    retry.style.cursor = "pointer";
    retry.onclick = function () {
      window.location.reload();
    };

    panel.appendChild(badge);
    panel.appendChild(heading);
    panel.appendChild(message);
    panel.appendChild(retry);
    root.appendChild(panel);
  }

  window.addEventListener(
    "error",
    function (event) {
      if (window.__LIFETRACE_MODULE_STARTED__) return;
      var target = event.target;
      if (target && target.tagName === "SCRIPT") {
        showFailure(
          "LifeTrace 桌面组件加载失败",
          "桌面 JavaScript 资源未能加载。请更新 Microsoft Edge WebView2 Runtime 后重试。",
        );
        return;
      }
      if (event.message) {
        showFailure("LifeTrace 桌面组件执行失败", String(event.message));
      }
    },
    true,
  );

  window.addEventListener("unhandledrejection", function (event) {
    if (window.__LIFETRACE_MODULE_STARTED__) return;
    var reason = event.reason;
    var detail = reason && reason.message ? reason.message : String(reason || "未知 JavaScript 错误");
    showFailure("LifeTrace 桌面组件执行失败", detail);
  });

  window.setTimeout(function () {
    showFailure(
      "LifeTrace 桌面组件加载超时",
      "启动入口在 15 秒内没有开始执行。通常是 WebView2 Runtime 过旧、脚本资源损坏或安装目录中的前端资源未能加载。",
    );
  }, FAILURE_DELAY_MS);
})();
