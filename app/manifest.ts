import type { MetadataRoute } from "next";

export default function manifest(): MetadataRoute.Manifest {
  return {
    id: "/fitness",
    name: "Life trace 导入",
    short_name: "Life trace",
    description: "从手机发送训练分享图和账单文件到电脑解析",
    start_url: "/fitness",
    scope: "/fitness",
    display: "standalone",
    background_color: "#f2f3ef",
    theme_color: "#14241f",
    orientation: "portrait",
    lang: "zh-CN",
    categories: ["productivity", "finance", "health"],
    icons: [
      { src: "/favicon.svg", sizes: "any", type: "image/svg+xml", purpose: "any" },
      { src: "/icons/icon-192.png", sizes: "192x192", type: "image/png", purpose: "any" },
      { src: "/icons/icon-512.png", sizes: "512x512", type: "image/png", purpose: "any" },
      { src: "/icons/icon-maskable-512.png", sizes: "512x512", type: "image/png", purpose: "maskable" },
    ],
    shortcuts: [{
      name: "数据导入",
      short_name: "导入",
      description: "上传训练数据和账单文件",
      url: "/fitness",
      icons: [{ src: "/icons/icon-192.png", sizes: "192x192", type: "image/png" }],
    }],
  };
}
