/** @type {import('next').NextConfig} */
const nextConfig = {
  async rewrites() {
    return [
      { source: '/api/rpc/:path*', destination: 'http://127.0.0.1:8899/:path*' },
    ];
  },
};
module.exports = nextConfig;
