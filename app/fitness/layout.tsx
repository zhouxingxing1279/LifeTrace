import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Life trace 导入",
  description: "从手机发送训记训练分享图和账单文件，由电脑端解析并入库。",
  applicationName: "Life trace 导入",
  manifest: "/manifest.webmanifest",
  appleWebApp: {
    capable: true,
    statusBarStyle: "black-translucent",
    title: "Life trace 导入",
  },
};

export default function FitnessLayout({ children }: { children: React.ReactNode }) {
  return children;
}
