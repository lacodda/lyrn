/*
 * The site agrees with itself about where it lives, or it does not build.
 *
 * A Starlight site is served in one of two ways, and each is internally
 * consistent:
 *
 *   - a github.io project site: `https://owner.github.io/repo/`, the
 *     repository's name as the base path, no public/CNAME;
 *   - a custom domain: `https://docs.example.com/`, served from the root,
 *     public/CNAME naming that domain.
 *
 * A mix of the two builds without complaint and breaks in the browser: with a
 * base path left over on a custom domain every asset is requested from a
 * directory that does not exist, and a link written from the root on a
 * github.io site leads off the site. Nothing in Astro or Starlight notices
 * either, because each value is valid on its own.
 *
 * So the build checks the combination, and every internal link in the pages
 * against the base path - Markdown links and the `link:` of a hero action are
 * written by hand and are not rewritten by anything.
 */
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';

/** Every link written from the root of the site, outside code, per page. */
export function rootLinks(source) {
	const outsideCode = source.replace(/```[\s\S]*?```/g, '').replace(/`[^`\n]*`/g, '');
	const found = [
		...outsideCode.matchAll(/\]\((\/[^)\s]*)\)/g),
		...outsideCode.matchAll(/^\s*(?:-\s+)?link:\s*['"]?(\/[^'"\s]*)/gm),
		...outsideCode.matchAll(/href="(\/[^"]*)"/g),
	].map((match) => match[1]);
	return [...new Set(found)];
}

/** What is wrong with where the site says it lives, as sentences. */
export function problems({ site, base, cname, pages }) {
	const found = [];
	if (!site) return ['`site` is not set in astro.config.mjs, so no page knows its own address'];

	const host = new URL(site).host;
	const root = base === '/' || base === '';
	const prefix = root ? '/' : `${base.replace(/\/$/, '')}/`;

	if (cname !== undefined) {
		if (cname !== host) {
			found.push(`public/CNAME says \`${cname}\` but the site is built for \`${host}\``);
		}
		if (!root) {
			found.push(
				`public/CNAME serves \`${cname}\` from the root, but the site is built under \`${base}\` - every asset would be requested from a path that does not exist`,
			);
		}
	} else if (host.endsWith('.github.io')) {
		if (root) {
			found.push(`\`${host}\` serves a project site under the repository's name, but the site has no base path`);
		}
	} else {
		found.push(`the site is built for \`${host}\`, a custom domain, but there is no public/CNAME naming it`);
	}

	if (!root) {
		for (const { slug, source } of pages) {
			for (const link of rootLinks(source)) {
				if (!link.startsWith(prefix) && `${link}/` !== prefix) {
					found.push(`${slug}: the link \`${link}\` leaves the site - it does not start with \`${prefix}\``);
				}
			}
		}
	}
	return found;
}

function readPages(dir, prefix = '') {
	if (!existsSync(dir)) return [];
	return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
		const path = join(dir, entry.name);
		if (entry.isDirectory()) return readPages(path, `${prefix}${entry.name}/`);
		if (!/\.mdx?$/.test(entry.name)) return [];
		return [{ slug: `${prefix}${entry.name}`, source: readFileSync(path, 'utf8') }];
	});
}

export default function address() {
	return {
		name: 'site-address',
		hooks: {
			'astro:config:done': ({ config }) => {
				const cnamePath = fileURLToPath(new URL('CNAME', config.publicDir));
				const cname = existsSync(cnamePath) ? readFileSync(cnamePath, 'utf8').trim() : undefined;
				const pages = readPages(fileURLToPath(new URL('content/docs/', config.srcDir)));

				const found = problems({ site: config.site, base: config.base, cname, pages });
				if (found.length) {
					throw new Error(`The site disagrees with itself about where it lives:\n  - ${found.join('\n  - ')}`);
				}
			},
		},
	};
}
