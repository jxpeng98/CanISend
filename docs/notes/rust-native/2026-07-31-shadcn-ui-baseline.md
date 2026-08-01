# shadcn-svelte system migration baseline

**Captured:** 2026-07-31

**Frontend:** `apps/canisend-desktop`

**Purpose:** Reproducible pre-migration visual reference for the UI0 state matrix.

The screenshots were captured from the local Vite fallback at the two supported desktop viewport
sizes. The fallback does not invoke Tauri mutations and therefore shows the empty-workspace state.

| Viewport | Theme | Language | Density | Text scale | Asset |
|---:|---|---|---|---:|---|
| 1280×820 | Light | English | Comfortable | 100% | [baseline](../../assets/ui-baseline/2026-07-31/desktop-1280-light-en-comfortable-100.png) |
| 1280×820 | Dark | Simplified Chinese | Compact | 100% | [baseline](../../assets/ui-baseline/2026-07-31/desktop-1280-dark-zh-compact-100.png) |
| 960×680 | Dark | Simplified Chinese | Compact | 200% | [baseline](../../assets/ui-baseline/2026-07-31/desktop-960-dark-zh-compact-200.png) |
| 960×680 | Light | English | Comfortable | 200% | [baseline](../../assets/ui-baseline/2026-07-31/desktop-960-light-en-comfortable-200.png) |

Together these four fixtures cover both themes, languages, densities, text-scale extremes, the
minimum supported window, and the default window. They are reference evidence, not pixel-perfect
goldens: the final visual suite owns stable component and screen assertions.

The development-only component gallery is available while Vite is running:

```text
http://127.0.0.1:1420/?ui-system=1
```

The gallery must never invoke Tauri commands or appear in the production bundle.
