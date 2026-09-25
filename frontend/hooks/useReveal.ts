"use client";

import { useEffect } from "react";

/**
 * Fades `.reveal` elements up as they scroll into view.
 *
 * Marks <html> with `reveal-ready` first, so the hidden starting state only
 * applies once this script runs; without JS every section stays visible.
 * Each element is revealed once and then unobserved.
 */
export function useReveal() {
  useEffect(() => {
    const root = document.documentElement;
    const elements = Array.from(document.querySelectorAll<HTMLElement>(".reveal"));

    if (!("IntersectionObserver" in window)) {
      elements.forEach((el) => el.classList.add("is-revealed"));
      return;
    }

    root.classList.add("reveal-ready");

    const reveal = (el: HTMLElement) => {
      el.classList.add("is-revealed");
      observer.unobserve(el);
    };

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          // Also reveal anything already scrolled past (restored scroll
          // position, anchor jump), which never crosses into view.
          if (!entry.isIntersecting && entry.boundingClientRect.top >= 0) return;
          const index = elements.indexOf(entry.target as HTMLElement);
          // A fast jump can skip a section without any callback firing for
          // it, so revealing one also reveals every section above it.
          elements.slice(0, index + 1).forEach(reveal);
        });
      },
      { rootMargin: "0px 0px -10% 0px", threshold: 0.1 },
    );

    elements.forEach((el) => observer.observe(el));

    return () => {
      observer.disconnect();
      root.classList.remove("reveal-ready");
    };
  }, []);
}
