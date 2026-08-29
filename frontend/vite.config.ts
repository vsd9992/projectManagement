import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    host: true,
    // Backend owns the /api prefix itself (see backend/api/src/lib.rs), so
    // this is a pure pass-through proxy — no path rewrite needed, and dev
    // matches whatever a future prod reverse proxy/ingress does.
    proxy: {
      '/api': {
        target: 'http://localhost:8081',
        changeOrigin: true,
      },
    },
  },
})
