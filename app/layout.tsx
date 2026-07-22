import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "LifeTrace — 长期生活记录",
  description: "本地优先的个人习惯、生活数据与每日复盘工具。",
  applicationName: "LifeTrace",
  manifest: "/manifest.webmanifest",
  icons: {
    icon: "/favicon.svg",
    shortcut: "/favicon.svg",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="zh-CN">
      <body>{children}</body>
    </html>
  );
}
