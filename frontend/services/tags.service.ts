import { Tag } from '../types/tag';
import { MOCK_TAGS } from '../mocks/tags.mock';
import { globalCache } from '../utils/cache';

const CACHE_TTL_SECONDS = 300; // 5 minutes
const MIN_LATENCY_MS = 10;
const MAX_LATENCY_MS = 30;

export async function getTags(prefix: string): Promise<Tag[]> {
  const normalizedPrefix = prefix.trim().toLowerCase();

  if (!normalizedPrefix) {
    return [];
  }

  const cacheKey = `tags:${normalizedPrefix}`;
  const cachedResult = globalCache.get<Tag[]>(cacheKey);

  if (cachedResult) {
    return cachedResult;
  }

  // Simulate network latency
  const latency = Math.floor(Math.random() * (MAX_LATENCY_MS - MIN_LATENCY_MS + 1)) + MIN_LATENCY_MS;
  await new Promise((resolve) => setTimeout(resolve, latency));

  // Filter, Sort, Deduplicate, Limit
  const filteredTags = MOCK_TAGS.filter((tag) =>
    tag.name.toLowerCase().includes(normalizedPrefix)
  );

  // Remove duplicates based on ID (though mock data shouldn't have them, good practice)
  // Also ensuring name uniqueness if IDs are different but names are same
  const uniqueTagsMap = new Map<string, Tag>();
  const uniqueNamesSet = new Set<string>();
  
  filteredTags.forEach((tag) => {
    if (!uniqueTagsMap.has(tag.id) && !uniqueNamesSet.has(tag.name.toLowerCase())) {
      uniqueTagsMap.set(tag.id, tag);
      uniqueNamesSet.add(tag.name.toLowerCase());
    }
  });
  
  const uniqueTags = Array.from(uniqueTagsMap.values());

  // Sort by usageCount DESC
  uniqueTags.sort((a, b) => b.usageCount - a.usageCount);

  // Limit to top 5
  const result = uniqueTags.slice(0, 5);

  // Cache the result
  globalCache.set(cacheKey, result, CACHE_TTL_SECONDS);

  return result;
}
