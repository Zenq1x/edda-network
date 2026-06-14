/** @type {import('next').NextConfig} */
const nextConfig = {
  async rewrites() {
    return [
      {
        source: '/api/rpc/:path*',
        destination: 'https://rpc.eddachain.com/:path*',
      },
    ];
  },
};

module.exports = nextConfig;
