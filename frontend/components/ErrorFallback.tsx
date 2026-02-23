'use client';

import { useState } from 'react';
import { AlertTriangle, RefreshCw, ChevronDown, ChevronUp } from 'lucide-react';
import { ErrorFallbackProps } from './ErrorBoundary';

export default function ErrorFallback({ error, errorInfo, resetError }: ErrorFallbackProps) {
  const [showDetails, setShowDetails] = useState(false);

  return (
    <div className="min-h-screen flex items-center justify-center p-4 bg-gray-50 dark:bg-gray-900">
      <div className="max-w-2xl w-full bg-white dark:bg-gray-800 rounded-lg shadow-xl p-8">
        <div className="flex items-start gap-4">
          <div className="flex-shrink-0">
            <AlertTriangle className="w-12 h-12 text-red-500" />
          </div>
          
          <div className="flex-1">
            <h1 className="text-2xl font-bold text-gray-900 dark:text-white mb-2">
              Something went wrong
            </h1>
            
            <p className="text-gray-600 dark:text-gray-300 mb-6">
              We encountered an unexpected error. Do not worry, your data is safe. 
              You can try refreshing the page or contact support if the problem persists.
            </p>

            <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4 mb-6">
              <p className="text-sm font-medium text-red-800 dark:text-red-200">
                {error.message || 'An unexpected error occurred'}
              </p>
            </div>

            <div className="flex gap-3 mb-6">
              <button
                onClick={resetError}
                className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors font-medium"
                aria-label="Try again"
              >
                <RefreshCw className="w-4 h-4" />
                Try Again
              </button>
              
              <button
                onClick={() => window.location.href = '/'}
                className="px-4 py-2 bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-900 dark:text-white rounded-lg transition-colors font-medium"
              >
                Go to Home
              </button>
            </div>

            <button
              onClick={() => setShowDetails(!showDetails)}
              className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200 transition-colors"
              aria-expanded={showDetails}
              aria-controls="error-details"
            >
              {showDetails ? (
                <>
                  <ChevronUp className="w-4 h-4" />
                  Hide technical details
                </>
              ) : (
                <>
                  <ChevronDown className="w-4 h-4" />
                  Show technical details
                </>
              )}
            </button>

            {showDetails && (
              <div
                id="error-details"
                className="mt-4 p-4 bg-gray-100 dark:bg-gray-900 rounded-lg overflow-auto max-h-96"
              >
                <div className="mb-4">
                  <h3 className="text-sm font-semibold text-gray-900 dark:text-white mb-2">
                    Error Details
                  </h3>
                  <pre className="text-xs text-gray-700 dark:text-gray-300 whitespace-pre-wrap break-words">
                    {error.stack || error.message}
                  </pre>
                </div>

                {errorInfo?.componentStack && (
                  <div>
                    <h3 className="text-sm font-semibold text-gray-900 dark:text-white mb-2">
                      Component Stack
                    </h3>
                    <pre className="text-xs text-gray-700 dark:text-gray-300 whitespace-pre-wrap break-words">
                      {errorInfo.componentStack}
                    </pre>
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
