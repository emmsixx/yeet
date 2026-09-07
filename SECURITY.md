# Security policy

## Supported versions

No public release is currently supported. During early development, fixes land
on `main`. Once releases begin, security fixes will target the latest stable
release; older versions are not promised backports.

## Reporting a vulnerability

Use the repository's **Security → Report a vulnerability** feature when available.
If private reporting is not enabled, contact the maintainer through their GitHub
profile to arrange a private channel. Do not open a public issue containing an
exploit, credentials, staged source, or private commit history.

Include the affected version and install method, platform, reproduction steps,
impact, and any proposed mitigation. There is no guaranteed response-time SLA.

## Security boundaries

The CLI stages all changes. Generated mode supplies staged content, and
repository-style mode also supplies sampled commit messages, to the selected
harness. That harness owns model-service authentication and data handling. Yeet
must not print full generation prompts in normal logs or treat repository text
as executable instructions.

Release binaries and installers are executable code. Downloads use HTTPS and
archive checksums; checksums detect corruption but do not independently prove
publisher identity. The release workflow also generates GitHub build provenance,
which can be verified separately. See [installation](docs/installation.md).

Normal Git commands must never silently update Yeet or contact an update server.
No telemetry or background updater is included in this release.
