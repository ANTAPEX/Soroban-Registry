'use client';

import Image from 'next/image';
import { useEffect, useState } from 'react';
import { ContractExample, api } from '@/lib/api';
import { generateBlurHashPlaceholder, generateSolidPlaceholder } from '@/lib/images';
import CodeRunner from './CodeRunner';
import { ThumbsUp, ThumbsDown } from 'lucide-react';

interface ExampleCardProps {
  example: ContractExample;
}

type ExampleWithOptionalImage = ContractExample & {
  repo_avatar_url?: string;
  repo_avatar_blurhash?: string;
  repo_avatar_placeholder_color?: string;
  thumbnail_url?: string;
  thumbnail_blurhash?: string;
  thumbnail_placeholder_color?: string;
};

export default function ExampleCard({ example }: ExampleCardProps) {
  const [activeTab, setActiveTab] = useState<'js' | 'rust'>('js');
  const [rating, setRating] = useState<number | null>(null); // Just for UI feedback
  const [isRating, setIsRating] = useState(false);
  const [avatarPlaceholder, setAvatarPlaceholder] = useState<string | null>(null);
  const [avatarLoadError, setAvatarLoadError] = useState(false);

  // If no JS code, default to Rust
  const effectiveTab = example.code_js ? activeTab : 'rust';
  const hasMultipleLangs = !!(example.code_js && example.code_rust);
  const exampleWithImage = example as ExampleWithOptionalImage;
  const avatarSrc = exampleWithImage.repo_avatar_url ?? exampleWithImage.thumbnail_url;
  const avatarBlurHash =
    exampleWithImage.repo_avatar_blurhash ?? exampleWithImage.thumbnail_blurhash;
  const avatarFallbackColor =
    exampleWithImage.repo_avatar_placeholder_color ??
    exampleWithImage.thumbnail_placeholder_color ??
    '#e5e7eb';

  useEffect(() => {
    setAvatarLoadError(false);

    if (!avatarSrc) {
      setAvatarPlaceholder(null);
      return;
    }

    if (avatarBlurHash) {
      setAvatarPlaceholder(
        generateBlurHashPlaceholder(avatarBlurHash, {
          width: 24,
          height: 24,
          fallbackColor: avatarFallbackColor,
        })
      );
      return;
    }

    setAvatarPlaceholder(generateSolidPlaceholder(avatarFallbackColor));
  }, [avatarSrc, avatarBlurHash, avatarFallbackColor]);

  const handleRate = async (val: number) => {
    try {
      setIsRating(true);
      // Generate a random user ID for demo purposes since we don't have auth yet
      const userId = localStorage.getItem('user_id') || Math.random().toString(36).substring(7);
      localStorage.setItem('user_id', userId);
      
      await api.rateExample(example.id, userId, val);
      setRating(val);
    } catch (err) {
      console.error('Failed to rate example', err);
    } finally {
      setIsRating(false);
    }
  };

  return (
    <div className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 overflow-hidden">
      <div className="p-6 border-b border-gray-200 dark:border-gray-800">
        <div className="flex items-start justify-between mb-4">
          <div className="flex items-start gap-3 min-w-0">
            {avatarSrc && !avatarLoadError ? (
              <div
                className="relative h-12 w-12 shrink-0 overflow-hidden rounded-full border border-gray-200 dark:border-gray-700 bg-gray-100 dark:bg-gray-800"
                style={{ backgroundColor: avatarFallbackColor }}
              >
                <Image
                  src={avatarSrc}
                  alt={`${example.title} repository avatar`}
                  fill
                  className="object-cover"
                  sizes="48px"
                  onError={() => setAvatarLoadError(true)}
                  placeholder={avatarPlaceholder ? 'blur' : 'empty'}
                  blurDataURL={avatarPlaceholder ?? undefined}
                />
              </div>
            ) : null}

            <div className="min-w-0">
              <h3 className="text-xl font-bold text-gray-900 dark:text-white mb-2 truncate">
                {example.title}
              </h3>
              <span className={`inline-block px-2 py-1 rounded text-xs font-medium uppercase tracking-wide ${
                example.category === 'basic' ? 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400' :
                example.category === 'advanced' ? 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-400' :
                'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400'
              }`}>
                {example.category}
              </span>
            </div>
          </div>
          
          <div className="flex items-center gap-2">
            <button
              onClick={() => handleRate(1)}
              disabled={isRating || rating === 1}
              className={`flex items-center gap-1 p-2 rounded-lg transition-colors ${
                rating === 1 ? 'bg-green-100 text-green-600' : 'hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-400'
              }`}
            >
              <ThumbsUp className="w-5 h-5" />
              <span className="text-sm font-medium">{example.rating_up + (rating === 1 ? 1 : 0)}</span>
            </button>
            <button
              onClick={() => handleRate(-1)}
              disabled={isRating || rating === -1}
              className={`flex items-center gap-1 p-2 rounded-lg transition-colors ${
                rating === -1 ? 'bg-red-100 text-red-600' : 'hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-400'
              }`}
            >
              <ThumbsDown className="w-5 h-5" />
              <span className="text-sm font-medium">{example.rating_down + (rating === -1 ? 1 : 0)}</span>
            </button>
          </div>
        </div>

        {example.description && (
          <p className="text-gray-600 dark:text-gray-300">
            {example.description}
          </p>
        )}
      </div>

      <div className="p-6">
        {hasMultipleLangs && (
          <div className="flex items-center gap-4 mb-4 border-b border-gray-200 dark:border-gray-800">
            <button
              onClick={() => setActiveTab('js')}
              className={`pb-2 text-sm font-medium transition-colors border-b-2 ${
                effectiveTab === 'js'
                  ? 'border-blue-600 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700'
              }`}
            >
              JavaScript / TypeScript
            </button>
            <button
              onClick={() => setActiveTab('rust')}
              className={`pb-2 text-sm font-medium transition-colors border-b-2 ${
                effectiveTab === 'rust'
                  ? 'border-blue-600 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700'
              }`}
            >
              Rust
            </button>
          </div>
        )}

        {effectiveTab === 'js' && example.code_js && (
          <CodeRunner
            initialCode={example.code_js}
            language="javascript"
            // Sent with copy events so analytics can identify the source example.
            copyAnalytics={{
              contractId: example.contract_id,
              exampleId: example.id,
              exampleTitle: example.title,
            }}
          />
        )}

        {effectiveTab === 'rust' && example.code_rust && (
          <CodeRunner
            initialCode={example.code_rust}
            language="rust"
            // Same metadata for Rust tab copies.
            copyAnalytics={{
              contractId: example.contract_id,
              exampleId: example.id,
              exampleTitle: example.title,
            }}
          />
        )}
      </div>
    </div>
  );
}
