import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

const api = process.env.FEUILLES_API ?? 'http://127.0.0.1:4000';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	optimizeDeps: {
		exclude: ['@facile/muse', '@ironcalc/wasm']
	},
	server: {
		proxy: {
			'/api': { target: api, changeOrigin: true, ws: true },
			'/version': { target: api, changeOrigin: true }
		}
	},
	test: {
		include: ['src/**/*.test.ts'],
		environment: 'node'
	}
});
