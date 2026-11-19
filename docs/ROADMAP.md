# Roadmap

The product documentation targets RaTeQ 1.0.0. These milestones organize development toward that specification. Adjust milestones as implementation progresses.

## Versioning

- Use Semantic Versioning for product releases.
- Record the package version in `Cargo.toml`, such as `0.1.0`, and tag its release as `v0.1.0`.
- During `0.x`, use minor releases for feature milestones and intentional compatibility changes. Use patches for compatible fixes.
- From `1.0.0`, use major releases for incompatible public behavior, minor releases for compatible additions, and patches for compatible fixes.
- Public behavior includes documented CLI commands, file interpretation, and interaction behavior.
- Use prerelease labels such as `1.0.0-beta.1` and `1.0.0-rc.1` for release validation.

Sources [Cargo versions](https://doc.rust-lang.org/cargo/reference/manifest.html#the-version-field), [Semantic Versioning](https://semver.org/).

## Milestones

| Stage | Target |
|---|---|
| PoC | Rendering, preview, input, and scheduling experiments. See [Commands](../poc/README.md). |
| `0.1.0` | Usable editor and RaTeX preview on the primary development terminal |
| `0.2.0` | Source save, PNG/SVG export, and file-entry flows |
| `0.3.0` | Determine the minimum terminal size and behavior below it, remaining modes, and Help in the target WezTerm environments |
| `0.4.0` | Persistence setup and resume |
| `1.0.0-beta.1` | Intended v1 features complete, gathering feedback |
| `1.0.0-rc.1` | Release validation and packaging |
| `1.0.0` | Documented v1 behavior and support targets fulfilled |

The PoC README lists experimental commands. Product releases follow the milestones above.

## Release Checks

- Build macOS and Linux `.tar.gz` archives with bundled assets and license notices.
- Before release, check the app in WezTerm on macOS and Ubuntu in OrbStack.
- Try relevant OS/terminal versions and record what worked and any limitations in [Support](README.md#release-targets).
- Smoke-test the exact release archives for startup, rendering, source saves, resume, exports, and Help.
