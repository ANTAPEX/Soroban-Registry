"use client";

import type { KeyboardEvent } from "react";
import { Check, Copy } from "lucide-react";
import { cn } from "@/lib/utils/cn";

interface CodeCopyButtonProps {
  onCopy: () => void | Promise<void> | Promise<boolean>;
  copied: boolean;
  disabled?: boolean;
  idleLabel?: string;
  copiedLabel?: string;
  shortcutHint?: boolean;
  /** "terminal" suits the dark code panels, which stay dark in both themes. */
  tone?: "default" | "terminal";
  className?: string;
}

export default function CodeCopyButton({
  onCopy,
  copied,
  disabled = false,
  idleLabel = "Copy",
  copiedLabel = "Copied",
  shortcutHint = true,
  tone = "default",
  className,
}: CodeCopyButtonProps) {
  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    const isCopyShortcut =
      (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "c";
    if (!isCopyShortcut || disabled) return;
    event.preventDefault();
    void onCopy();
  };

  const shortcutLabel = shortcutHint ? " (Ctrl/Cmd+C)" : "";

  return (
    <button
      type="button"
      onClick={() => void onCopy()}
      onKeyDown={handleKeyDown}
      disabled={disabled}
      className={cn(
        "inline-flex items-center gap-1 rounded-md border px-2.5 py-1 text-xs font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-60",
        tone === "terminal"
          ? "border-white/10 bg-transparent text-deep-foreground/60 hover:bg-white/10 hover:text-deep-foreground"
          : "border-border bg-card text-foreground hover:bg-accent",
        className,
      )}
      aria-label={`${copied ? copiedLabel : idleLabel}${shortcutLabel}`}
      title={`${copied ? copiedLabel : idleLabel}${shortcutLabel}`}
    >
      {copied ? (
        <Check className="h-3.5 w-3.5" />
      ) : (
        <Copy className="h-3.5 w-3.5" />
      )}
      {copied ? copiedLabel : idleLabel}
    </button>
  );
}
