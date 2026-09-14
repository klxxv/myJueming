This directory receives generated, platform-specific research resources from
`scripts/prepare-research-release.py` or `scripts/build-research-bundle.py`.
Runtime, model and manifests are build artifacts and must not be committed.
A full research release must include `bundle.json` (single architecture) or
`bundle.<arch>.json` (Universal), and every integrity-covered file.
See `docs/architecture/local-research-runtime-implementation-v0.1.md` for current
runtime and release boundaries. Cloud/account/licensing services are not required.
