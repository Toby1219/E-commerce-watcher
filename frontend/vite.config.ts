import react, { reactCompilerPreset } from '@vitejs/plugin-react'
import babel from '@rolldown/plugin-babel'
import { defineConfig } from 'vite'
import tailwindcss from "@tailwindcss/vite"

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    babel({ presets: [reactCompilerPreset()] })
  ],
  server: {
    host: 'localhost', // Or '0.0.0.0' if running in Docker/VM
    port: 5173,
    hmr: {
      protocol: 'ws',
      host: 'localhost',
      port: 5173,
    },
    proxy:{
      '/api': {
        target: 'http://localhost:8181',
        changeOrigin: true,
      },
      '/auth': {
        target: 'http://localhost:8181',
        changeOrigin: true,
      },
    }
  },
})
