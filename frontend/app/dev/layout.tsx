import type { Metadata } from "next";

// app/dev/breakers and app/dev/error-test are "use client" pages, so they
// can't export their own `metadata` — the override has to live in a Server
// Component layout for the segment instead. Without this, both inherit the
// sitewide `index: true, follow: true` from the root layout and are public
// and indexable, despite being internal debug scaffolding.
export const metadata: Metadata = {
  robots: {
    index: false,
    follow: false,
  },
};

export default function DevLayout({ children }: { children: React.ReactNode }) {
  return children;
}
