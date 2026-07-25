import type { Metadata, Viewport } from "next";
import PwaManager from "@/src/components/PwaManager";
import "./globals.css";
import "./hengxu.css";
import "./fitness-app.css";
import "./english.css";
import "./xunji-import.css";
import "./notes.css";
import "./redesign.css";

export const metadata: Metadata = {
  metadataBase: new URL("https://lifetrace-personal.zxxzxxzxx.chatgpt.site"),
  title: "Life trace — 个人管理平台",
  description: "将坚持、训练、财务与复盘整合在一起的本机 SQLite 个人管理系统。",
  applicationName: "Life trace",
  openGraph: {
    title: "Life Trace — 个人管理平台",
    description: "把每一天，沉淀成自己的轨迹。",
    type: "website",
    locale: "zh_CN",
    images: [{ url: "/og.png", width: 1733, height: 907, alt: "Life Trace 个人管理平台" }],
  },
  twitter: {
    card: "summary_large_image",
    title: "Life Trace — 个人管理平台",
    description: "把每一天，沉淀成自己的轨迹。",
    images: ["/og.png"],
  },
  appleWebApp: {
    capable: true,
    statusBarStyle: "black-translucent",
    title: "Life trace",
  },
  formatDetection: {
    telephone: false,
  },
  icons: {
    icon: [
      { url: "/favicon.svg", type: "image/svg+xml" },
      { url: "/icons/icon-192.png", sizes: "192x192", type: "image/png" },
      { url: "/icons/icon-512.png", sizes: "512x512", type: "image/png" },
    ],
    shortcut: "/favicon.svg",
    apple: [{ url: "/icons/apple-touch-icon.png", sizes: "180x180", type: "image/png" }],
  },
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  viewportFit: "cover",
  themeColor: "#111c2d",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="zh-CN">
      <body>
        {children}
        <PwaManager />
      </body>
    </html>
  );
}
