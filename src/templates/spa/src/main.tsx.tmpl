import { StrictMode, type ReactNode } from 'react'
import { createRoot } from 'react-dom/client'
{{#tanstack-query}}
import { QueryClientProvider } from '@tanstack/react-query'
{{/tanstack-query}}
{{#router}}
import { BrowserRouter } from 'react-router'
{{/router}}
import { App } from '@/App'
{{#tanstack-query}}
import { queryClient } from '@/lib/query'
{{/tanstack-query}}
{{#auth}}
import { SessionProvider } from '@/lib/session'
{{/auth}}
{{#pwa}}
import { registerServiceWorker } from '@/pwa'
{{/pwa}}
import '@/styles.css'

const container = document.getElementById('root')
if (!container) {
  throw new Error('The #root element is missing from index.html')
}

/*
 * What the application is wrapped in, outermost first. A provider is one line
 * here, so adding or removing one never re-indents the others.
 */
const wrappers: ((children: ReactNode) => ReactNode)[] = [
  (children) => <StrictMode>{children}</StrictMode>,
{{#tanstack-query}}
  (children) => <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>,
{{/tanstack-query}}
{{#router}}
  (children) => <BrowserRouter>{children}</BrowserRouter>,
{{/router}}
{{#auth}}
  (children) => <SessionProvider>{children}</SessionProvider>,
{{/auth}}
]

createRoot(container).render(wrappers.reduceRight<ReactNode>((children, wrap) => wrap(children), <App />))
{{#pwa}}
registerServiceWorker()
{{/pwa}}
