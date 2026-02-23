interface CacheItem<T> {
  value: T;
  expiry: number;
}

export class InMemoryCache {
  private cache: Map<string, CacheItem<any>>;

  constructor() {
    this.cache = new Map();
  }

  set<T>(key: string, value: T, ttlSeconds: number): void {
    const expiry = Date.now() + ttlSeconds * 1000;
    this.cache.set(key, { value, expiry });
  }

  get<T>(key: string): T | null {
    const item = this.cache.get(key);
    if (!item) {
      return null;
    }

    if (Date.now() > item.expiry) {
      this.cache.delete(key);
      return null;
    }

    return item.value;
  }

  clear(): void {
    this.cache.clear();
  }
}

export const globalCache = new InMemoryCache();
