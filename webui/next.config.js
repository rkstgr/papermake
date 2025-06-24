/** @type {import('next').NextConfig} */
const nextConfig = {
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: process.env.API_BASE_URL ? `${process.env.API_BASE_URL}/api/:path*` : 'http://localhost:3001/api/:path*',
      },
    ]
  },
}

module.exports = nextConfig
