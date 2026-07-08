import type { Metadata, Viewport } from "next";
import type { ReactNode } from "react";
import "./globals.css";

export const metadata: Metadata = {
  title: "Greybound | Open-source greybox audio experiment",
  description:
    "Greybound is an open-source experiment for greybox guitar tone: circuit ideas, measured behavior, Rust DSP, and transparent model notes.",
  icons: {
    icon: "/greybound-robine-mark.svg",
  },
};

export const viewport: Viewport = {
  themeColor: "#121417",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: ReactNode;
}>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
