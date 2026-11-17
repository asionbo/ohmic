# Changelog

## [0.1.8] - 2026-11-17

### Fixed

- **Stale QR link after network changes.** The LAN IP was detected once at
  startup and never refreshed, so after an overnight DHCP renewal, a Wi-Fi
  switch or a wake-from-sleep the QR page and the tray "Copy phone link"
  still pointed at the old address — and the `Host` check rejected the new
  IP too, so phones could not connect at all. Now:
  - a background thread re-enumerates LAN candidates every 30 s;
  - when the selected IP disappears the app switches to the best remaining
    candidate automatically (a manually chosen IP is kept while it stays up);
  - opening the `/pair` page refreshes the network first, so the QR always
    reflects the current address;
  - the tray "Network" submenu is rebuilt when the interface list changes,
    and the tray status shows a "network address changed, rescan" notice;
  - the `Host` check now accepts any candidate IP instead of only the one
    selected at startup.

## [0.1.7] — 2026-11-12

### Added

- Structured file logging (`~/Library/Caches/ohmic/logs/ohmic.log` on macOS)
  so fatal errors are captured even when stderr is unavailable for a GUI
  process.
- Watchdog thread: probes the main runloop every 20 s and exits with code 97
  if the loop stops consuming events for ~60 s, preventing silent one-core
  hangs (2026-11-11 incident).

## [0.1.6] — 2026-11-11

### Added

- Tray menu entries for the GitHub repository and the app version.

## [0.1.5] — 2026-11-11

### Changed

- Renamed the project from **nomic** to **ohmic**.
- Updated GitHub Actions runner versions (macOS 15).

## [0.1.4] — 2026-11-06

### Changed

- Updated tray icons.

## [0.1.3] — 2026-11-06

### Added

- "Press Enter after paste" toggle in the tray menu.

## [0.1.2] — 2026-11-05

### Added

- VitePress documentation site under `site/`.

### Fixed

- Lint issues.

## [0.1.1] — 2026-11-05

### Changed

- macOS event loop configuration so the app stays out of the Dock
  (`LSUIElement` accessory policy).

## [0.1.0] — 2026-11-05

### Added

- Initial release: phone web page types into the desktop clipboard and pastes
  at the cursor over a token-protected LAN connection, with QR pairing,
  tray-only app, clipboard-only fallback, and EN/ZH UI.

[0.1.8]: https://github.com/asionbo/ohmic/compare/v0.1.7...HEAD
[0.1.7]: https://github.com/asionbo/ohmic/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/asionbo/ohmic/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/asionbo/ohmic/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/asionbo/ohmic/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/asionbo/ohmic/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/asionbo/ohmic/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/asionbo/ohmic/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/asionbo/ohmic/releases/tag/v0.1.0
