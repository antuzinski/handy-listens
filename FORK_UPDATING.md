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

## Upstream monitoring and code review

The `Check upstream changes` GitHub Actions workflow runs on weekdays at 08:23
UTC (11:23 Moscow time) and can also be started from **Actions → Check upstream
changes → Run workflow**. For each source,
`.github/upstream-baselines.json` pins both the reviewed release tag and commit
SHA. The workflow compares the latest release with the release baseline and
the source's default branch with the commit baseline, so commits made after a
release are visible too.

When it finds a newer release or commits, it creates or updates a GitHub Issue
with the `upstream-review` label. The issue includes the reviewed baseline,
current upstream commit IDs, release and comparison links, commit summaries,
and changed paths. This is a review packet, not an AI code review: it does not
judge semantic compatibility, run a build, or copy/merge code. The maintainer
decides what to port, verifies the changes, and then advances both the release
and commit fields for that source in the baseline file.

Klava's history has diverged from Handy, so its report can include commits from
the shared base. Follow the Klava overlay procedure below and do not merge the
Klava branch wholesale. After review, update that source's baseline and close
the issue with a short list of changes accepted or skipped.
