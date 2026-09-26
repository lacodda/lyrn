# {{ title }}

{{ description }}

Built on the lacodda line's stack: [Vite](https://vite.dev), React, TypeScript,
[Tailwind](https://tailwindcss.com) and the [dowel](https://lacodda.github.io/dowel)
design system. Scaffolded with [lyrn](https://github.com/lacodda/lyrn).

## A day in the life

```console
$ pnpm install          # once
$ pnpm dev              # the app, on http://localhost:5173
$ pnpm lint             # eslint + types + tests, the same gate CI runs
$ pnpm build            # a production bundle in dist/
```

## Adding a component

Primitives are copied in from the dowel registry into `src/components/ui/`:

```console
$ npx shadcn add {{ registry }}/badge.json
```

A lowercase file there is dowel's copy and stays dowel's: `pnpm lint` compares
each one with the registry of the dowel-ui this project installed, so an
upgrade that forgets to take the copies again fails the gate. A component of
this project's own is named in PascalCase. The full set is listed at
<https://lacodda.github.io/dowel>.

## The look

This project states one thing about its own appearance — its accent, in
`src/styles.css`:

```css
:root {
  --accent-base: {{ accent }};
}
```

Everything else follows from it: the hover shade, the soft fill, the focus
ring, the tint the greys carry, and what colour text sits on an accent fill.

A component never writes a colour down; it names a token, so the theme can swap
it underneath. `pnpm lint` enforces this.
{{#router}}

## Screens

Screens live at addresses, through [React Router](https://reactrouter.com).
`src/App.tsx` lists them - a route is one line there and one component in
`src/pages/` - and an address nothing answers goes home rather than to a blank
page. A reload on a deep address needs the server to answer every path with
`index.html`; `pnpm dev` and `pnpm preview` already do.
{{/router}}
{{#tanstack-query}}

## Server state

Data from a server goes through [TanStack Query](https://tanstack.com/query):
one client in `src/lib/query.ts`, on the line's defaults - fresh for 30
seconds, no refetch on every return to the window, and a failed query fails at
once so the screen can say so instead of spinning through three retries.
{{/tanstack-query}}
{{#auth}}

## Signing in

The session is an HttpOnly cookie, out of reach of any script on the page, so
the app asks the server who is signed in rather than remembering it. Until
somebody is, it shows the sign-in screen. It expects three endpoints:

| Endpoint | Answers |
| --- | --- |
| `GET /api/me` | `{ "email", "name" }`, or 401 when nobody is signed in |
| `POST /api/auth/login` | `{ "email", "password" }` in; 204 and the cookie, or 401 |
| `POST /api/auth/logout` | 204, with the cookie cleared |

In development `/api` is forwarded to `localhost:8080`, where the line's
service form listens.
{{/auth}}
{{#pwa}}

## Installing it

The app installs from the browser and opens without a network:
`public/manifest.webmanifest` names it and its icons, and `public/sw.js`
serves the last build it saw when the network is gone - network first, so a
deploy shows on the next load. The API is never cached: yesterday's data shown
as today's is worse than an error. The worker is registered in production
builds only, where each build gets a cache of its own.
{{/pwa}}

## Layout

| Path | What lives there |
| --- | --- |
| `src/` | The application |
| `src/App.tsx` | Which screen is showing |
| `src/pages/` | The screens |
| `src/components/ui/` | dowel's primitives, copied from the registry |
| `src/lib/utils.ts` | `cn()`, the class-name joiner the primitives use |
{{#tanstack-query}}
| `src/lib/query.ts` | The query client and its defaults |
{{/tanstack-query}}
{{#auth}}
| `src/lib/api.ts`, `src/lib/session.tsx` | The API client and who is signed in |
| `src/test/session.tsx` | The server, as tests stand in for it |
{{/auth}}
| `src/styles.css` | Tailwind, the dowel theme, this product's accent |
| `public/favicon.svg` | The mark at level S; the line's umbrella mark until this product has one |
{{#pwa}}
| `public/manifest.webmanifest`, `public/sw.js` | What an install reads, and what serves offline |
{{/pwa}}
| `tools/check-registry.mjs` | Holds the copies - primitives and favicon - to the installed dowel-ui |
| `docs/adr/` | Architecture decisions, newest last |
| `.github/workflows/ci.yml` | The gate: lint, types, tests, build |

## License

MIT — see [LICENSE](LICENSE).
