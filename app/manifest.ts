import type { MetadataRoute } from "next";

export default function manifest(): MetadataRoute.Manifest {
  return { name: "LifeTrace 长期生活记录", short_name: "LifeTrace", description: "本地优先的个人习惯与生活数据工具", start_url: "/", display: "standalone", background_color: "#f4f3ef", theme_color: "#1f6b5c", icons: [{ src: "/favicon.svg", sizes: "any", type: "image/svg+xml" }] };
}
