import type { NextConfig } from "next";
import withBundleAnalyzer from '@next/bundle-analyzer';

const apiOrigin = process.env.API_URL || process.env.NEXT_PUBLIC_API_URL || "http://localhost:3001";

const analyzer = withBundleAnalyzer({
  enabled: process.env.ANALYZE === 'true',
});

const nextConfig: NextConfig = {
  output: 'standalone',
  images: {
    // Enable modern image formats (WebP, AVIF) with fallbacks
    formats: ['image/avif', 'image/webp'],
    // Allow external images from common sources
    remotePatterns: [
      {
        protocol: 'https',
        hostname: 'avatars.githubusercontent.com',
        pathname: '/**',
      },
      {
        protocol: 'https',
        hostname: 'github.com',
        pathname: '/**',
      },
      {
        protocol: 'https',
        hostname: '*.githubusercontent.com',
        pathname: '/**',
      },
      {
        protocol: 'https',
        hostname: 'gravatar.com',
        pathname: '/**',
      },
      {
        protocol: 'https',
        hostname: '*.stellar.org',
        pathname: '/**',
      },
      {
        protocol: 'https',
        hostname: 'ipfs.io',
        pathname: '/**',
      },
      {
        protocol: 'https',
        hostname: 'arweave.net',
        pathname: '/**',
      },
      {
        protocol: 'https',
        hostname: 'ui-avatars.com',
        pathname: '/api/**',
      },
    ],
    // Define device sizes for responsive images
    deviceSizes: [640, 750, 828, 1080, 1200, 1920, 2048, 3840],
    // Define image sizes for srcset
    imageSizes: [16, 32, 48, 64, 96, 128, 256, 384],
    // Minimum cache TTL for optimized images (seconds)
    minimumCacheTTL: 60 * 60 * 24 * 30, // 30 days
  },
  async rewrites() {
    return [
      {
        source: "/api/:path*",
        destination: `${apiOrigin}/api/:path*`,
      },
    ];
  },
};

export default analyzer(nextConfig);
