"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useAppDispatch, useAppSelector } from "@/store/hooks";
import { setFavorites } from "@/store/slices/favoritesSlice";
import { useToast } from "./useToast";
import { usePreferences, useUpdatePreferences } from "./queries/preferences";

const STORAGE_KEY = "soroban_registry_favorites";
const AUTH_TOKEN_KEY = "soroban_registry_token";
const MAX_FAVORITES = 500;

function deduplicate(arr: string[]): string[] {
  return arr.filter((id, index) => arr.indexOf(id) === index);
}

function getAuthToken(): string | null {
  if (typeof window === "undefined") return null;
  try {
    return localStorage.getItem(AUTH_TOKEN_KEY);
  } catch {
    return null;
  }
}

function readFromLocalStorage(): string[] {
  if (typeof window === "undefined") return [];
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) return [];
    const parsed = JSON.parse(stored);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((item): item is string => typeof item === "string");
  } catch {
    return [];
  }
}

/**
 * Whether localStorage can actually be written to.
 *
 * This used to be a `try {} catch {}` around an empty block, so the warning
 * below it could never fire. Private windows and blocked site data both make
 * the write throw, which is the case worth telling someone about.
 */
function isStorageAvailable(): boolean {
  if (typeof window === "undefined") return false;
  try {
    const probe = `${STORAGE_KEY}__probe`;
    localStorage.setItem(probe, "1");
    localStorage.removeItem(probe);
    return true;
  } catch {
    return false;
  }
}

function writeToLocalStorage(favorites: string[]): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(favorites));
  } catch {
    // ignore
  }
}

export function useFavorites() {
  const dispatch = useAppDispatch();
  const items = useAppSelector((s) => s.favorites.items);
  const { showError, showWarning } = useToast();

  const [token, setToken] = useState<string | null>(null);
  const prevTokenRef = useRef<string | null>(null);

  const { data: preferences, isLoading, isError } = usePreferences(token);
  const updatePreferences = useUpdatePreferences(token);

  // The token lives in localStorage, so it is read once on mount rather than
  // during render; `usePreferences` stays disabled until it exists.
  useEffect(() => {
    const initial = getAuthToken();
    prevTokenRef.current = initial;
    setToken(initial);
    if (!initial) {
      if (!isStorageAvailable()) {
        showWarning("Favorites won't be saved — browser storage is unavailable");
      }
      dispatch(setFavorites(deduplicate(readFromLocalStorage())));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // The server's answer, or the local copy if the request failed.
  useEffect(() => {
    if (!token) return;
    if (preferences) {
      dispatch(setFavorites(deduplicate(preferences.favorites)));
    } else if (isError) {
      dispatch(setFavorites(deduplicate(readFromLocalStorage())));
    }
  }, [dispatch, token, preferences, isError]);

  // Auth change detection (merge on login, clear on logout).
  useEffect(() => {
    const checkAuthChange = () => {
      const currentToken = getAuthToken();
      const prevToken = prevTokenRef.current;
      if (currentToken === prevToken) return;

      prevTokenRef.current = currentToken;
      setToken(currentToken);

      if (!currentToken && prevToken) {
        dispatch(setFavorites([]));
        try {
          localStorage.removeItem(STORAGE_KEY);
        } catch {}
      }
    };

    const interval = setInterval(checkAuthChange, 1000);
    const onStorage = (e: StorageEvent) => {
      if (e.key === AUTH_TOKEN_KEY) checkAuthChange();
    };
    window.addEventListener("storage", onStorage);
    return () => {
      clearInterval(interval);
      window.removeEventListener("storage", onStorage);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Guest -> authenticated: the local list and the server's are merged, and the
  // result is written back. `usePreferences` did the fetch; this only reacts to
  // its answer arriving for a token that was not there before.
  const mergedForTokenRef = useRef<string | null>(null);
  useEffect(() => {
    if (!token || !preferences) return;
    if (mergedForTokenRef.current === token) return;
    mergedForTokenRef.current = token;

    const local = readFromLocalStorage();
    if (local.length === 0) return;

    const merged = deduplicate([...local, ...preferences.favorites]).slice(
      0,
      MAX_FAVORITES,
    );
    dispatch(setFavorites(merged));
    writeToLocalStorage(merged);
    updatePreferences.mutate(merged);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token, preferences, dispatch]);

  const toggleFavorite = useCallback(
    (id: string) => {
      const previous = items;
      const optimistic = previous.includes(id)
        ? previous.filter((i) => i !== id)
        : deduplicate([...previous, id]).slice(0, MAX_FAVORITES);

      dispatch(setFavorites(optimistic));
      writeToLocalStorage(optimistic);

      if (!getAuthToken()) return;

      // The mutation retries once after three seconds on its own; this only
      // has to undo the optimistic write when that retry fails too.
      updatePreferences.mutate(optimistic, {
        onError: () => {
          dispatch(setFavorites(previous));
          writeToLocalStorage(previous);
          showError("Failed to save favorites. Please try again.");
        },
      });
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [dispatch, items, showError],
  );

  const isFavorited = useCallback((id: string) => items.includes(id), [items]);

  const clearAllFavorites = useCallback(() => {
    dispatch(setFavorites([]));
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {}
    if (!getAuthToken()) return;
    updatePreferences.mutate([], {
      onError: () =>
        showError("Failed to clear favorites. Please try again."),
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dispatch, showError]);

  return {
    favorites: items,
    toggleFavorite,
    isFavorited,
    favoritesCount: items.length,
    isLoading,
    clearAllFavorites,
  };
}
