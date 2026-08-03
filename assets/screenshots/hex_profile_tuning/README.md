# Hex deformation tuning — screenshots

Native captures from the tuning run (`tune(map): restore monotonic hex
deformation profiles`, fc950b5), a native window capture at the game's
default boot view on the canonical 40x40 map. Use the same camera/view for
side-by-side comparison.

| File | Intent |
| ---- | ------ |
| `00_boot.png` | Boot frame before any profile cycling. |
| `01_subtle_wide.png` | Default profile (Subtle; expected ~0.100 average displacement). |
| `02_organic_wide.png` | After one profile advance (Organic; expected ~0.151, low-frequency flow + high-frequency detail). |
| `03_pagonia_wide.png` | After two advances (PagoniaLike; expected ~0.201, strongest warp). |

Notes:

- Captured with a screen grab while the app was focused; frames were not
  visually audited by an automated agent (image inspection unavailable), so
  confirm they show the deformation overlay as expected.
- The intended 8-frame set (wide + close-ups + regular/warped outlines +
  shared vertices in Pago) is not fully covered: the overlay panel toggles
  and camera zoom must be driven interactively. Missing frames:
  `04_subtle_close`, `05_organic_close`, `06_pago_close`,
  `07_pago_outlines`, `08_pago_shared`.
- These documents the *measured* separation contract enforced by tests
  (per-seed gaps >= 0.015, 256-seed min gap ~0.049); the screenshots are
  evidence only, not a substitute for the numeric checks.