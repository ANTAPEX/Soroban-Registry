"use client";

import React, { Component, ReactNode } from "react";
import ErrorFallback from "./ErrorFallback";
import { logError } from "@/lib/errors";

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: React.ComponentType<ErrorFallbackProps>;
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: React.ErrorInfo | null;
}

export interface ErrorFallbackProps {
  error: Error;
  errorInfo: React.ErrorInfo | null;
  resetError: () => void;
}

const EXTENSION_URL = /(chrome|moz|safari(-web)?)-extension:\/\//;

/**
 * Wallet and other browser extensions inject scripts into the page, and
 * their failures surface on window. They are not the app's errors, so
 * they are not worth logging.
 */
function isFromBrowserExtension(err: Error, filename?: string): boolean {
  return EXTENSION_URL.test(filename ?? "") || EXTENSION_URL.test(err.stack ?? "");
}

export default class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    };
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return {
      hasError: true,
      error,
    };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    // Log error to console with component stack
    logError(error, {
      componentStack: errorInfo.componentStack,
      errorBoundary: true,
    });

    // Store error info in state
    this.setState({ errorInfo });

    // Call optional error callback
    if (this.props.onError) {
      this.props.onError(error, errorInfo);
    }
  }

  componentDidMount() {
    // Log uncaught errors and promise rejections at the window level. They
    // are logged only: an error here did not break rendering (render errors
    // reach getDerivedStateFromError), and many come from browser extensions
    // or background requests, so replacing the page for them does harm.
    if (typeof window !== "undefined") {
      window.addEventListener("error", this.handleGlobalError as EventListener);
      window.addEventListener(
        "unhandledrejection",
        this.handleUnhandledRejection as EventListener,
      );
    }
  }

  componentWillUnmount() {
    if (typeof window !== "undefined") {
      window.removeEventListener(
        "error",
        this.handleGlobalError as EventListener,
      );
      window.removeEventListener(
        "unhandledrejection",
        this.handleUnhandledRejection as EventListener,
      );
    }
  }

  handleGlobalError = (event: ErrorEvent) => {
    try {
      const err =
        event.error || new Error(event.message || "Unknown window error");
      if (isFromBrowserExtension(err, event.filename)) return;
      logError(err, {
        source: "window.error",
        filename: event.filename,
        lineno: event.lineno,
        colno: event.colno,
      });

    } catch {
      // swallow to avoid infinite loops
    }
  };

  handleUnhandledRejection = (event: PromiseRejectionEvent) => {
    try {
      const reason = event.reason;
      const err =
        reason instanceof Error
          ? reason
          : new Error(
              typeof reason === "string" ? reason : JSON.stringify(reason),
            );
      if (isFromBrowserExtension(err)) return;
      logError(err, { source: "unhandledrejection" });
    } catch {
      // swallow
    }
  };

  resetError = () => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
    });
  };

  render() {
    if (this.state.hasError && this.state.error) {
      const FallbackComponent = this.props.fallback || ErrorFallback;

      return (
        <FallbackComponent
          error={this.state.error}
          errorInfo={this.state.errorInfo}
          resetError={this.resetError}
        />
      );
    }

    return this.props.children;
  }
}
