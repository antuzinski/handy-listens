# Maintaining this Handy fork

This checkout is the `vscg/Handy` live-captions fork, merged with upstream
`cjpais/Handy` v0.9.7. Keep future upstream updates as merge commits so the
fork's history and changes remain visible; do not replace this branch with an
upstream checkout or install an official Handy updater package over it.

The in-app updater is intentionally disabled for Windows builds and its
upstream release endpoint is removed. To update this fork, fetch an upstream
release tag, merge that tag into the fork branch, resolve conflicts while
preserving the fork's live/system-audio capture, combined mic+system capture,
Whisper prompt/post-processing, custom-word behavior, and theme/settings UI,
then run the Rust tests, frontend build, and a Windows release build. Keep the
previous fork binary and the settings/history/recordings data untouched until
the new build has been checked.
