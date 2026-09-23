// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

import address from './src/integrations/address.mjs';
import llms from './src/integrations/llms.mjs';

// Said once: the site, llms.txt and the page metadata all read these.
const title = 'lyrn';
const description = "Start a new web application on the lacodda line's stack with one command.";

// Served from the custom domain in ./public/CNAME, so the site sits at the
// root - no `base` path, unlike a github.io project site.
export default defineConfig({
	site: 'https://lyrn.lacodda.com',
	integrations: [
		address(),
		starlight({
			title,
			description,
			// The 404 page is a content page (src/content/docs/404.md): Starlight's
			// own route looks for that entry and warns on every build without it.
			disable404Route: true,
			logo: {
				src: './src/assets/logo.svg',
				alt: title,
			},
			favicon: '/favicon.svg',
			customCss: ['./src/styles/brand.css'],
			head: [
				{ tag: 'link', attrs: { rel: 'apple-touch-icon', href: '/apple-touch-icon.png' } },
				{ tag: 'meta', attrs: { property: 'og:image', content: 'https://raw.githubusercontent.com/lacodda/lyrn/main/assets/social-preview.png' } },
				{ tag: 'meta', attrs: { name: 'twitter:card', content: 'summary_large_image' } },
			],
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/lacodda/lyrn' }],
			editLink: {
				baseUrl: 'https://github.com/lacodda/lyrn/edit/main/docs/',
			},
			sidebar: [
				{ label: 'Getting Started', slug: 'getting-started' },
				{
					label: 'Concepts',
					items: [{ autogenerate: { directory: 'concepts' } }],
				},
				{
					label: 'Reference',
					items: [{ autogenerate: { directory: 'reference' } }],
				},
			],
		}),
		llms({ title, description }),
	],
});
