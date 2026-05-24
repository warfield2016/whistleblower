/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,

  // WASM support: Next.js 14 needs both client and server bundlers configured.
  webpack: (config, { isServer }) => {
    config.experiments = {
      ...config.experiments,
      asyncWebAssembly: true,
      layers: true,
    };
    // Don't try to polyfill 'fs' etc when the WASM is imported in client code.
    if (!isServer) {
      config.resolve.fallback = {
        ...config.resolve.fallback,
        fs: false,
        path: false,
      };
    }
    return config;
  },
};

module.exports = nextConfig;
