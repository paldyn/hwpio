import { defineConfig } from 'vite';
import { resolve } from 'path';

// rhwp-studio를 Firefox 확장용으로 빌드
// 산출물: rhwp-firefox/dist/ → viewer.html + JS/CSS + WASM + 폰트
export default defineConfig({
  root: resolve(__dirname, '..', 'rhwp-studio'),
  publicDir: false, // public/ 폴더 제외 (samples, images 등 불필요)
  resolve: {
    alias: {
      '@': resolve(__dirname, '..', 'rhwp-studio', 'src'),
      '@wasm': resolve(__dirname, '..', 'pkg'),
    },
  },
  build: {
    outDir: resolve(__dirname, 'dist'),
    emptyDir: true,
    rollupOptions: {
      input: {
        viewer: resolve(__dirname, '..', 'rhwp-studio', 'index.html'),
      },
    },
    // WASM inline 방지 — 별도 파일로 유지
    assetsInlineLimit: 0,
  },
  // 개발 서버 (확장 디버깅용)
  server: {
    host: '0.0.0.0',
    port: 7702,
    fs: {
      allow: ['..'],
    },
  },
});
