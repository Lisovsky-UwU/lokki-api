# Contributing

Thanks for taking an interest in LokkiAPI. This file describes how work moves through the repository: branches, commits, checks and releases. For setting the project up and running it, see [Development](README.md#development).

## Branching model

One permanent branch, `master`, plus short-lived branches for individual changes. There is no `develop`: a released version is a tag with binaries attached to it, so a second long-lived branch would only duplicate `master` with a delay.

| Branch | Lifetime | Purpose |
|---|---|---|
| `master` | permanent | Always builds, always green. Every release is tagged from it |
| `feat/…`, `fix/…`, `chore/…`, `docs/…` | hours to days | One change. Deleted once merged |
| `release/X.Y.x` | as needed | Only to patch a version that is already published while `master` has moved on. Never created in advance |

Keep branches short. The longer one lives apart from `master`, the more the merge costs, and nothing about a long-lived branch pays that back.

## Making a change

```bash
git switch master && git pull
git switch -c feat/openapi-import
# work, commit
git push -u origin feat/openapi-import
```

Then open a pull request. It needs no reviewer, but it does need the three CI checks to pass — that is the point of the PR: the frontend type check and both `cargo test` runs see the change before `master` does. Merge with **Squash and merge**, so the history of `master` stays a list of finished changes.

## Commits

One commit, one change, and a subject line that says what was done rather than that something was improved. `Cancel request button` is useful a year later; `Improvements and bug fixing` is not.

Prefixing the subject with `feat:`, `fix:`, `chore:` or `docs:` is encouraged. It costs nothing and it is what makes a changelog something you generate rather than something you reconstruct by reading the log.

## Before opening a pull request

Run both checks locally — they are the same ones CI runs:

```bash
npm run check
cd src-tauri && cargo test
```

A few project rules are not enforced by the compiler and are the usual reason a change comes back:

- **No user-facing string is written inline.** Every one of them, in Svelte components and in the messages Rust commands return, comes from a catalogue: `src/lib/i18n/{en,ru}.ts` and `src-tauri/src/i18n/messages.rs`. A key added to `en.ts` and forgotten in `ru.ts` fails `npm run check`; the Rust side will not build without both halves of `tr!`.
- **`src/lib/bindings/types.ts` mirrors the `domain` structs by hand.** Changing a field there changes the on-disk TOML format, so both sides move together, and field names stay `snake_case`.
- **A change to one README belongs in the other.** `README.md` and `README.ru_RU.md` are the same document in two languages, except that the roadmap lives only in the English one — so a box cannot be ticked in one language and not the other.
- **Comments and docs use the short dash (`-`).** The long dash belongs only inside user-facing strings in the i18n catalogues, where it is typography rather than punctuation.

## Releasing

Releases are tagged straight from `master` — no release branch is involved.

```bash
git switch master && git pull
# raise `version` in src-tauri/Cargo.toml
(cd src-tauri && cargo check)            # refreshes Cargo.lock
npm version 0.3.0 --no-git-tag-version   # package.json + package-lock.json
git commit -am "v0.3.0"
git push
git tag v0.3.0 && git push origin v0.3.0
```

`src-tauri/Cargo.toml` is the one version that matters: `tauri.conf.json` carries none, so Tauri falls back to it for the installer names, the package metadata and the About dialog. The npm files are cosmetic but should not drift.

Pushing the tag starts `release.yml`, which builds all four bundles and opens a draft release. Write the changelog into it and publish.

Only what is already in `master` gets released. A change that did not make it goes in the next version rather than onto the tag.

## Patching a released version

When a bug is found in a published version and `master` has already moved on, fix it on `master` first, then carry it back — that way the fix cannot be lost on the next release.

```bash
# 1. fix on master through the usual pull request

# 2. carry it to a branch cut from the tag
git switch -c release/0.3.x v0.3.0
git cherry-pick <the fix commit from master>
# raise the version to 0.3.1 in the same files as above
git commit -am "v0.3.1"
git push -u origin release/0.3.x
git tag v0.3.1 && git push origin v0.3.1
```

`release.yml` triggers on any `v*` tag regardless of the branch it points at, so the patch release builds with no extra setup.
