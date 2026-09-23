/*
 * The documentation, in the form a machine reads.
 *
 * Three artefacts, written into the built site from the same pages the site
 * is built from, so there is no second copy of the docs to go stale:
 *
 *   /llms.txt       - the index: what the product is, and a link per page
 *   /llms-full.txt  - every page's prose, in one document
 *   /<page>.md      - each page as plain Markdown, beside its HTML
 *
 * They are made after `astro build` has written the site, into its output,
 * rather than into public/: generated files kept in the source tree are a
 * second copy, and a second copy is what goes stale.
 *
 * A page may render components. Dropping their tags would hand the reader a
 * page with a hole in it, so each one Starlight ships is turned into what it
 * says - a card into a heading and its text, a link card into a link. A tag
 * with no rule fails the build and names itself: the rule belongs here, and
 * until it exists the Markdown twin would be missing whatever the component
 * shows.
 */
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

/** Frontmatter fields and the body, from one page's source. */
export function parse(source) {
	const text = source.replace(/\r\n/g, '\n');
	const match = text.match(/^---\n([\s\S]*?)\n---\n?([\s\S]*)$/);
	if (!match) throw new Error('a documentation page has no frontmatter');
	const [, frontmatter, body] = match;

	const field = (key) => {
		const line = frontmatter.match(new RegExp(`^\\s*${key}:\\s*(.*)$`, 'm'));
		if (!line) return '';
		const value = line[1].trim();
		// A double-quoted value may carry escapes; read it as the JSON it is.
		if (value.startsWith('"')) {
			try {
				return JSON.parse(value);
			} catch {
				return value.slice(1, -1);
			}
		}
		return value.replace(/^'|'$/g, '');
	};

	const splash = /^template:\s*splash/m.test(frontmatter);
	return { title: field('title'), description: field('description'), tagline: splash ? field('tagline') : '', body };
}

/*
 * Code is held aside for the whole pass: the rules below rewrite tags and
 * imports, and examples are made of exactly those.
 */
function withCodeHeld(text, transform) {
	const held = [];
	const hidden = text.replace(/```[\s\S]*?```|`[^`\n]*`/g, (code) => {
		held.push(code);
		return `\u0000${held.length - 1}\u0000`;
	});
	return transform(hidden).replace(/\u0000(\d+)\u0000/g, (_, index) => held[Number(index)]);
}

const attr = (props, name) => props.match(new RegExp(`${name}="([^"]*)"`))?.[1] ?? '';

/** The body with MDX turned into something readable on its own. */
export function expand(body) {
	return withCodeHeld(body, (text) => {
		let out = text;

		// Imports are machinery; what they bring in is handled below.
		out = out.replace(/^import .*(\n|$)/gm, '');

		// Containers whose meaning is layout alone.
		out = out.replace(/<\/?(CardGrid|Steps|Tabs|FileTree)(\s[^>]*)?>/g, '');

		// A card and a tab each title the text they hold.
		out = out.replace(/<Card\s([^>]*)>/g, (_, props) => `### ${attr(props, 'title')}\n`);
		out = out.replace(/<TabItem\s([^>]*)>/g, (_, props) => `**${attr(props, 'label')}**\n`);
		out = out.replace(/<\/(Card|TabItem)>/g, '');

		// An aside is a note set apart; a blockquote is the Markdown for that.
		out = out.replace(/<Aside([^>]*)>([\s\S]*?)<\/Aside>/g, (_, props, inner) => {
			const title = attr(props, 'title') || attr(props, 'type') || 'note';
			const lines = inner.trim().split('\n').map((line) => `> ${line.trim()}`);
			return `> **${title[0].toUpperCase()}${title.slice(1)}:**\n${lines.join('\n')}`;
		});

		// A link card is a link with a sentence.
		out = out.replace(/<LinkCard\s([^>]*?)\/>/g, (_, props) => {
			const description = attr(props, 'description');
			return `- [${attr(props, 'title')}](${attr(props, 'href')})${description ? `: ${description}` : ''}`;
		});

		// A badge is its text; an icon is decoration.
		out = out.replace(/<Badge\s([^>]*?)\/>/g, (_, props) => `(${attr(props, 'text')})`);
		out = out.replace(/<Icon\s[^>]*?\/>/g, '');

		// Leading tabs are MDX indentation, which Markdown would read as code.
		out = out.replace(/^\t+/gm, '');

		return out.replace(/\n{3,}/g, '\n\n').trim();
	});
}

/** Component tags left outside code: each one is content the twin lost. */
export function remainingComponents(body) {
	const outsideCode = body.replace(/```[\s\S]*?```/g, '').replace(/`[^`\n]*`/g, '');
	return [...new Set([...outsideCode.matchAll(/<\/?([A-Z][A-Za-z]*)[\s/>]/g)].map((m) => m[1]))];
}

/** Every page, as `{ slug, title, description, body }`, sorted by slug. */
export function pages(contentDir) {
	const found = [];
	const walk = (dir, prefix) => {
		for (const entry of readdirSync(dir, { withFileTypes: true })) {
			if (entry.isDirectory()) {
				walk(join(dir, entry.name), `${prefix}${entry.name}/`);
				continue;
			}
			if (!/\.mdx?$/.test(entry.name)) continue;
			const name = entry.name.replace(/\.mdx?$/, '');
			// The 404 page is where a lost reader lands, not documentation.
			if (!prefix && name === '404') continue;

			const page = parse(readFileSync(join(dir, entry.name), 'utf8'));
			const body = expand(page.body);
			found.push({
				slug: name === 'index' ? `${prefix.replace(/\/$/, '')}` : `${prefix}${name}`,
				title: page.title,
				description: page.description,
				// A splash page carries its opening in frontmatter; without it the
				// body starts mid-thought.
				body: [page.tagline, body].filter(Boolean).join('\n\n'),
			});
		}
	};
	walk(contentDir, '');
	return found.sort((a, b) => a.slug.localeCompare(b.slug));
}

/*
 * Sections follow the directories, so a page cannot belong to none of them:
 * the front page and Getting Started open the index, every directory is a
 * section named after it.
 */
export function sections(all) {
	const start = all.filter((page) => page.slug === '' || page.slug === 'getting-started');
	const rest = all.filter((page) => !start.includes(page));
	const groups = new Map();
	for (const page of rest) {
		const key = page.slug.includes('/') ? page.slug.split('/')[0] : '';
		if (!groups.has(key)) groups.set(key, []);
		groups.get(key).push(page);
	}
	const heading = (key) => (key ? key[0].toUpperCase() + key.slice(1).replace(/-/g, ' ') : 'More');
	return [
		...(start.length ? [['Start here', start]] : []),
		...[...groups.keys()].sort().map((key) => [heading(key), groups.get(key)]),
	];
}

/** The three artefacts, as `{ path: text }`. */
export function render({ title, description, home, all }) {
	const url = (page) => `${home}${page.slug ? `${page.slug}.md` : 'index.md'}`;
	const source = (page) => `${home}${page.slug ? `${page.slug}/` : ''}`;
	const grouped = sections(all);

	const index = [
		`# ${title}`,
		'',
		`> ${description}`,
		'',
		...grouped.flatMap(([heading, list]) => [
			`## ${heading}`,
			'',
			...list.map((page) => `- [${page.title}](${url(page)})${page.description ? `: ${page.description}` : ''}`),
			'',
		]),
		'## Full text',
		'',
		`- [llms-full.txt](${home}llms-full.txt): every page above, in one document`,
		'',
	].join('\n');

	const twin = (page) => [`# ${page.title}`, '', `Source: ${source(page)}`, '', page.body, ''].join('\n');

	const full = [
		`# ${title}`,
		'',
		`> ${description}`,
		'',
		...grouped.flatMap(([, list]) => list.flatMap((page) => ['---', '', twin(page)])),
	].join('\n');

	const files = { 'llms.txt': index, 'llms-full.txt': full };
	for (const page of all) files[page.slug ? `${page.slug}.md` : 'index.md'] = twin(page);
	return files;
}

export default function llms({ title, description }) {
	let home;
	let contentDir;
	return {
		name: 'llms-txt',
		hooks: {
			'astro:config:done': ({ config }) => {
				home = `${new URL(config.base, config.site ?? 'http://localhost/').href.replace(/\/?$/, '/')}`;
				contentDir = fileURLToPath(new URL('content/docs/', config.srcDir));
			},
			'astro:build:done': ({ dir, logger }) => {
				const all = pages(contentDir);

				const lost = all.flatMap((page) => remainingComponents(page.body).map((tag) => `${page.slug || 'index'}: <${tag}>`));
				if (lost.length) {
					throw new Error(
						`These components have no Markdown rule in src/integrations/llms.mjs, so their content would be missing from the .md twins and llms.txt:\n  - ${lost.join('\n  - ')}`,
					);
				}

				const out = fileURLToPath(dir);
				const files = render({ title, description, home, all });
				for (const [path, text] of Object.entries(files)) {
					const target = join(out, path);
					mkdirSync(dirname(target), { recursive: true });
					writeFileSync(target, text);
				}
				logger.info(`${all.length} pages as Markdown, llms.txt and llms-full.txt (${(files['llms-full.txt'].length / 1024).toFixed(0)} kB)`);
			},
		},
	};
}
