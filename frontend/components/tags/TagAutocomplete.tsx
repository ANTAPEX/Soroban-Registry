'use client';

import React, { useState, useEffect, useRef, ChangeEvent } from 'react';
import { Tag } from '../../types/tag';
import { getTags } from '../../services/tags.service';
import { Search, Loader2, X } from 'lucide-react';

interface TagAutocompleteProps {
  onSelect?: (tag: Tag) => void;
  onClear?: () => void;
  placeholder?: string;
  className?: string;
}

export default function TagAutocomplete({
  onSelect,
  onClear,
  placeholder = 'Search tags...',
  className = '',
}: TagAutocompleteProps) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<Tag[]>([]);
  const [loading, setLoading] = useState(false);
  const [isOpen, setIsOpen] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);
  
  const wrapperRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Close dropdown when clicking outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (wrapperRef.current && !wrapperRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, []);

  // Debounced search
  useEffect(() => {
    const timer = setTimeout(async () => {
      const normalizedQuery = query.trim();
      
      if (normalizedQuery.length === 0) {
        setResults([]);
        setIsOpen(false);
        setHasSearched(false);
        return;
      }

      setLoading(true);
      setHasSearched(true);
      try {
        const tags = await getTags(normalizedQuery);
        setResults(tags);
        setIsOpen(true);
      } catch (error) {
        setResults([]);
      } finally {
        setLoading(false);
      }
    }, 300);

    return () => clearTimeout(timer);
  }, [query]);

  const handleInputChange = (e: ChangeEvent<HTMLInputElement>) => {
    setQuery(e.target.value);
  };

  const handleSelect = (tag: Tag) => {
    setQuery(tag.name);
    setIsOpen(false);
    if (onSelect) {
      onSelect(tag);
    }
  };

  const clearSearch = () => {
    setQuery('');
    setResults([]);
    setIsOpen(false);
    setHasSearched(false);
    onClear && onClear();
    inputRef.current?.focus();
  };

  // Helper to highlight matching text
  const highlightMatch = (text: string, highlight: string) => {
    if (!highlight.trim()) {
      return <span>{text}</span>;
    }
    const regex = new RegExp(`(${highlight.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    const parts = text.split(regex);
    return (
      <span>
        {parts.map((part, i) => 
          regex.test(part) ? (
            <span key={i} className="font-bold text-blue-600 bg-blue-50 rounded-sm px-0.5">
              {part}
            </span>
          ) : (
            <span key={i}>{part}</span>
          )
        )}
      </span>
    );
  };

  return (
    <div ref={wrapperRef} className={`relative w-full max-w-md ${className}`}>
      <div className="relative">
        <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
          <Search className="h-4 w-4 text-gray-400" />
        </div>
        <input
          ref={inputRef}
          type="text"
          className="block w-full pl-10 pr-10 py-2 border border-gray-300 rounded-md leading-5 bg-white placeholder-gray-500 focus:outline-none focus:placeholder-gray-400 focus:border-blue-500 focus:ring-1 focus:ring-blue-500 sm:text-sm transition duration-150 ease-in-out"
          placeholder={placeholder}
          value={query}
          onChange={handleInputChange}
          onFocus={() => {
            if (results.length > 0 && query.trim().length > 0) setIsOpen(true);
          }}
          aria-expanded={isOpen}
          aria-autocomplete="list"
          role="combobox"
          aria-controls="tag-results"
        />
        <div className="absolute inset-y-0 right-0 pr-3 flex items-center">
             {loading ? (
                <Loader2 className="h-4 w-4 text-gray-400 animate-spin" />
             ) : query ? (
                <X className="h-4 w-4 text-gray-400 hover:text-gray-600 cursor-pointer" onClick={clearSearch} />
             ) : null}
        </div>
      </div>

      {isOpen && (
        <ul
          id="tag-results"
          className="absolute z-10 mt-1 w-full bg-white shadow-lg max-h-60 rounded-md py-1 text-base ring-1 ring-black ring-opacity-5 overflow-auto focus:outline-none sm:text-sm"
          role="listbox"
        >
          {results.length > 0 ? (
            results.map((tag) => (
              <li
                key={tag.id}
                className="cursor-pointer select-none relative py-2 pl-3 pr-4 hover:bg-gray-100 text-gray-900 group"
                role="option"
                onClick={() => handleSelect(tag)}
              >
                <div className="flex justify-between items-center">
                  <span className="block truncate">
                    {highlightMatch(tag.name, query)}
                  </span>
                  <span className="text-xs text-gray-500 group-hover:text-gray-700">
                    ({tag.usageCount} uses)
                  </span>
                </div>
              </li>
            ))
          ) : (
             hasSearched && !loading && (
              <li className="cursor-default select-none relative py-2 pl-3 pr-9 text-gray-500 italic">
                No matching tags
              </li>
             )
          )}
        </ul>
      )}
    </div>
  );
}
