import React from "react";
import { PublisherResponse } from "@/types/publisher";
import { Calendar, ExternalLink, Github, Globe, CheckCircle } from "lucide-react";
import Link from "next/link";

interface PublisherHeaderProps {
  publisher: PublisherResponse;
}

export function PublisherHeader({ publisher }: PublisherHeaderProps) {
  const formattedDate = new Date(publisher.createdAt).toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-6 md:p-8">
      <div className="flex flex-col md:flex-row items-start md:items-center gap-6">
        {/* Avatar */}
        <div className="relative shrink-0">
          <img
            src={publisher.avatarUrl || `https://ui-avatars.com/api/?name=${publisher.displayName}`}
            alt={publisher.displayName}
            className="w-24 h-24 md:w-32 md:h-32 rounded-full border-4 border-gray-100 dark:border-gray-700 object-cover bg-gray-100 dark:bg-gray-800"
          />
        </div>

        {/* Info */}
        <div className="flex-1 w-full">
          <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 mb-2">
            <div>
              <h1 className="text-2xl md:text-3xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
                {publisher.displayName}
                {publisher.verifiedContracts > 0 && (
                  <CheckCircle className="w-6 h-6 text-blue-500" aria-label="Verified Publisher" />
                )}
              </h1>
              <p className="text-sm text-gray-500 dark:text-gray-400 font-mono mt-1 break-all">
                {publisher.address}
              </p>
            </div>
            
            <button
              disabled
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-colors text-sm w-full md:w-auto"
              title="Coming soon"
            >
              Follow Publisher
              {/* TODO: Integrate with follow API in future */}
            </button>
          </div>

          {publisher.bio && (
            <p className="text-gray-600 dark:text-gray-300 mb-4 max-w-2xl leading-relaxed">
              {publisher.bio}
            </p>
          )}

          <div className="flex flex-wrap items-center gap-4 text-sm text-gray-500 dark:text-gray-400">
            <div className="flex items-center gap-1.5">
              <Calendar className="w-4 h-4" />
              <span>Joined {formattedDate}</span>
            </div>

            {publisher.website && (
              <a
                href={publisher.website}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-1.5 hover:text-blue-600 dark:hover:text-blue-400 transition-colors"
                aria-label="Visit website"
              >
                <Globe className="w-4 h-4" />
                <span>Website</span>
                <ExternalLink className="w-3 h-3" />
              </a>
            )}

            {publisher.github && (
              <a
                href={publisher.github}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-1.5 hover:text-gray-900 dark:hover:text-white transition-colors"
                aria-label="Visit GitHub profile"
              >
                <Github className="w-4 h-4" />
                <span>GitHub</span>
                <ExternalLink className="w-3 h-3" />
              </a>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
