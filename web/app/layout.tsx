import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Whistleblower · censorship-resistant document publication on Logos",
  description:
    "Browser demo of the upload → broadcast → anchor pipeline. WASM-compiled Rust running the same wire format as the production Basecamp module.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className="antialiased">{children}</body>
    </html>
  );
}
