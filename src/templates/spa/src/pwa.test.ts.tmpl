import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'
import { registerServiceWorker } from '@/pwa'

const PUBLIC = join(process.cwd(), 'public')

interface Manifest {
  start_url: string
  display: string
  icons: { src: string; sizes: string; type: string }[]
}

const manifest = JSON.parse(readFileSync(join(PUBLIC, 'manifest.webmanifest'), 'utf8')) as Manifest

describe('the web app manifest', () => {
  // An install prompt needs both, and a phone's home screen takes the larger.
  it('offers the two icon sizes an install needs', () => {
    const sizes = manifest.icons.map((icon) => icon.sizes)
    expect(sizes).toContain('192x192')
    expect(sizes).toContain('512x512')
  })

  // A manifest pointing at an icon that is not there installs with a blank
  // one, and nothing on the page says so.
  it('points only at icons that exist', () => {
    for (const icon of manifest.icons) {
      expect(existsSync(join(PUBLIC, icon.src)), icon.src).toBe(true)
    }
  })

  it('opens as an application, at the start', () => {
    expect(manifest.display).toBe('standalone')
    expect(manifest.start_url).toBe('/')
  })
})

describe('registerServiceWorker', () => {
  it('stays out of the way outside a production build', () => {
    expect(registerServiceWorker()).toBe(false)
  })
})
