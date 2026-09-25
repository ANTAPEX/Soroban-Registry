"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactNode, useState } from "react";
import { ThemeProvider } from "@/providers/ThemeProvider";
import ToastProvider from "@/providers/ToastProvider";
import RealtimeProvider from "@/providers/RealtimeProvider";
import ErrorBoundary from "./ErrorBoundary";
import LanguageDirSync from "./LanguageDirSync";
import { CookiesProvider } from "react-cookie";
import { ApiError } from "@/lib/errors";

/** Client errors (404 and friends) will not change on retry; timeouts and rate limits can. */
function shouldRetry(failureCount: number, error: unknown): boolean {
  const status = error instanceof ApiError ? error.statusCode : undefined;
  if (status && status >= 400 && status < 500 && status !== 408 && status !== 429) {
    return false;
  }
  return failureCount < 3;
}

// Redux
import { Provider as ReduxProvider } from "react-redux";
import { PersistGate } from "redux-persist/integration/react";
import { store, persistor } from "@/store";

export default function Providers({ children }: { children: ReactNode }) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 60 * 1000, // 1 minute
            refetchOnWindowFocus: false,
            retry: shouldRetry,
          },
        },
      }),
  );

  return (
    <ErrorBoundary>
      <LanguageDirSync />
      <ReduxProvider store={store}>
        <PersistGate loading={null} persistor={persistor}>
          <CookiesProvider>
            <QueryClientProvider client={queryClient}>
              <ThemeProvider>
                <RealtimeProvider>
                  <ToastProvider>{children}</ToastProvider>
                </RealtimeProvider>
              </ThemeProvider>
            </QueryClientProvider>
          </CookiesProvider>
        </PersistGate>
      </ReduxProvider>
    </ErrorBoundary>
  );
}
