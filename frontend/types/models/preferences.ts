/**
 * User preference types for Soroban Registry
 */

export interface UserPreferences {
  favorites: string[];
  // Add other preference fields as needed
  [key: string]: unknown;
}
