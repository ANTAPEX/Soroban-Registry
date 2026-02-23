import type { Metadata } from "next";
import "./globals.css";
import Providers from "@/components/Providers";
import Script from "next/script";
import PageViewTracker from "@/components/PageViewTracker";
import UserInteractionTracker from "@/components/UserInteractionTracker";

const GA_PROVIDER = process.env.NEXT_PUBLIC_ANALYTICS_PROVIDER || 'ga'
const GA_ID = process.env.NEXT_PUBLIC_GA_ID

export const metadata: Metadata = {
  title: "Soroban Registry - Smart Contract Discovery for Stellar",
  description: "Discover, publish, and verify Soroban smart contracts on the Stellar network. The trusted registry for Stellar developers.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <head>
        {/* Only load GA script if GA is selected */}
        {GA_PROVIDER === 'ga' && GA_ID && (
          <>
            <Script
              strategy="afterInteractive"
              src={`https://www.googletagmanager.com/gtag/js?id=${GA_ID}`}
            />
            <Script
              id="ga-init"
              strategy="afterInteractive"
              dangerouslySetInnerHTML={{
                __html: `
                  window.dataLayer = window.dataLayer || [];
                  function gtag(){dataLayer.push(arguments);}
                  gtag('js', new Date());
                  gtag('config', '${GA_ID}', { send_page_view: false });
                `,
              }}
            />
          </>
        )}
        {/* You could similarly inject Plausible or Mixpanel scripts here if needed */}
      </head>
      <body className="font-sans antialiased">
        <Providers>
          {children}

          {/* called on every page to track page views */}
          <PageViewTracker />
          {/* tracks external link clicks, form submissions, and client runtime errors */}
          <UserInteractionTracker />
        </Providers>
      </body>
    </html>
  )
}
