import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      '/login': { target: 'http://127.0.0.1:8080', changeOrigin: true },
      '/register': { target: 'http://127.0.0.1:8080', changeOrigin: true },
      '/rooms': { target: 'http://127.0.0.1:8080', changeOrigin: true },
      '/ws': { target: 'ws://127.0.0.1:8080', ws: true },
      '/health': { target: 'http://127.0.0.1:8080', changeOrigin: true },
      '/users': { target: 'http://127.0.0.1:8080', changeOrigin: true },
      '/games': { target: 'http://127.0.0.1:8080', changeOrigin: true },
    },
  },
  build: {
    cssCodeSplit: false,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('/games/lincoln/')) return 'game-lincoln';
          if (id.includes('/games/texasHoldem/')) return 'game-texas';
          if (id.includes('/games/werewolf/')) return 'game-werewolf';
          if (id.includes('/games/blackjack/')) return 'game-blackjack';
        },
      },
    },
  },
})
