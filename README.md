<p align="center">
  <img src="docs/logo-banner.png" alt="LokkiAPI" width="620">
</p>

<p align="center">
  A local, file-based API client — a lightweight alternative to Postman and Insomnia.
</p>

<p align="center">
  <a href="README.ru_RU.md">Русская версия</a>
</p>

---

## About

LokkiAPI does three things differently from the clients already out there.

**Your data is plain files, not a hidden database.** Every request is its own TOML file inside a collection folder. You can read them, put them in git, review them in pull requests, sync them however you already sync files. No export/import dance to share a collection with your team.

**The app is light.** Tauri with the system WebView instead of Electron: less space disk, and an order of magnitude less memory than clients that ship their own Chromium.

**The architecture is built to grow.** Every entity carries a stable ULID and a version, so a future self-hosted sync server plugs in without migrating the format. Request execution sits behind a `ProtocolExecutor` trait, so WebSocket, SSE and GraphQL are added alongside HTTP rather than by rewriting the core.

**No subscriptions, ever, and open source forever.** LokkiAPI is built on open code and the absence of subscriptions of any kind. Never, in any form. Every feature is free to use for any person and any company.

## Features

**Requests**
- GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS
- Query parameters and headers, each row individually enabled or disabled
- Body: JSON, arbitrary text, form (urlencoded)
- Auth: none, Bearer token, Basic
- Preview of the final URL with variables substituted, and a copy button
- Ctrl+Enter to send, Ctrl+S to save

**Collections**
- Nested folders, any depth
- Drag and drop to reorder and to move between folders and collections; the collections themselves are reordered the same way
- Rename and delete from the ⋯ menu, and expand or collapse a whole collection from it
- The order of folders and requests is set by hand and stored in the files

**Importing**
- OpenAPI 3.x and Swagger 2.0, JSON or YAML, from a file or by link
- Operations become requests grouped into folders by tag; every server in the specification becomes an environment of its own, and the first is made active
- Parameters, headers, an example body built from the schema and the authorization scheme all come across; whatever could not be reproduced is listed when the import is done
- The import always creates a new collection, so nothing you already have can be overwritten

**Environments and variables**
- Global environments and collection environments; the collection wins over the global one
- `{{variable}}` substitution in the URL, query parameters, headers, body and auth
- Variable completion on `{{`, showing the value and the scope
- A warning on the response when a variable could not be resolved
- Secret variables are stored outside the syncable tree

**Response**
- Status, duration, body size and the time it ran
- Body and Headers tabs, headers as a table
- JSON syntax highlighting in the GitHub palette, response formatting
- The last response is remembered per request

**Interface**
- English and Russian: picked from the OS language on first launch, falling back to English; switchable under Settings → Interface
- Light and dark theme: follows the system by default, or pin either one under Settings → Interface
- Request and response either one above the other or side by side, chosen under Settings → Interface
- Resizable panes: collection tree width, and the split between request and response
- The tree remembers which collections and folders were open
- The last workspace reopens on launch, or the welcome screen does — chosen under Settings → Interface
- The welcome screen lists the last 10 workspaces by name and path; each can be removed from the list or shown in the file manager

## Installing

Builds for all three platforms are attached to every release — take them from the [latest one](https://github.com/Lisovsky-UwU/lokki-api/releases/latest). Nothing has to be installed alongside the app: it draws its interface through the WebView the system already ships, and keeps your data in a workspace folder you pick yourself.

**Windows x64**
- `…_x64-setup.exe` — the ordinary installer
- `…_x64_en-US.msi` — for deployment through group policy

**Linux x86_64**
- `.deb` — Debian, Ubuntu, Mint: `sudo apt install ./<the downloaded file>`
- `.rpm` — Fedora, openSUSE: `sudo dnf install ./<the downloaded file>`
- `.AppImage` — any distribution, Arch included: make it executable and run it

**macOS**
- the `aarch64` `.dmg` — Apple Silicon
- the `x64` `.dmg` — Intel

The builds are not code-signed yet. Windows SmartScreen hides "Run anyway" behind "More info", and macOS calls the app damaged until the quarantine flag is cleared with `xattr -cr /Applications/LokkiAPI.app`.

Building it yourself instead is two commands — see [Development](#development).

## What a workspace looks like on disk

A workspace is an ordinary folder you pick on first launch. A new one has to
go into an empty folder — a workspace takes the whole folder over, so the
picker refuses anything that already holds files. Opening an existing
workspace has no such restriction.

```
my-workspace/
  .lokki/
    workspace.toml          # workspace identity and schema version
    secrets.local.toml      # values of secret variables (gitignored)
    .gitignore              # written automatically
  environments/
    Global.env.toml         # global environments
  Petstore/                 # a collection
    collection.toml
    environments/
      Dev.env.toml          # collection environments
    Pets/                   # a folder
      folder.toml           # the folder's name and position
      List Pets.lokki.toml  # a request
```

A request file:

```toml
[meta]
id = "01J8XA1B2C3D4E5F6G7H8J9K0M"
name = "List Pets"
seq = 1
protocol = "http"
created_at = "2026-08-01T10:05:00Z"
updated_at = "2026-09-05T09:12:44Z"
version = 5

[http]
method = "GET"
url = "{{baseUrl}}/pets"

[[http.headers]]
key = "Accept"
value = "application/json"
enabled = true

[http.auth]
type = "bearer"
token = "{{authToken}}"

[http.body]
type = "none"
```

TOML rather than YAML or JSON: it has no implicit type coercion and no indentation traps when edited by hand, it gives line-by-line diffs on arrays of tables, and it has multi-line literals for request bodies.

### Secrets

A variable carries a `secret` flag. The value of such a variable is **never** written to the environment file — an empty string stays there instead, and the value itself lives in `.lokki/secrets.local.toml`, which is added to `.gitignore` automatically. That is what makes the whole workspace folder safe to commit without leaking tokens.

Variable names are limited to `A-Z a-z 0-9 _ . -`. Characters outside that set never resolve at send time, so the input field refuses them.

## Development

You need Rust stable, Node.js LTS and Tauri's system dependencies (on Windows that is WebView2, which ships with Windows 11).

```bash
npm install
npm run tauri dev        # run in development mode
npm run check            # frontend type check
npm run tauri build      # build the installers
cd src-tauri && cargo test   # core tests
```

Helper examples for exercising the core without the UI:

```bash
cd src-tauri
cargo run --example smoke              # end-to-end run with a real HTTP request
cargo run --example fixture -- <path>   # generate a demo workspace
cargo run --example import_spec -- <spec>  # print what an OpenAPI import would build
```

The app icons are regenerated from the source image with one command:

```bash
npx tauri icon lokki_api_logo.png
```

Before opening a pull request, read [CONTRIBUTING.md](CONTRIBUTING.md) — it covers branches, commit messages, the checks that have to pass and how a release is cut.

## Architecture

| Layer | Technology |
|---|---|
| Shell | Tauri 2, system WebView |
| Core | Rust: domain, file storage, request execution, interpolation |
| Interface | Svelte 5 + TypeScript, CodeMirror 6 |
| Transport | reqwest with rustls, no OpenSSL dependency |

Core modules (`src-tauri/src`):

| Module | Purpose |
|---|---|
| `domain/` | Models: workspace, collection, folder, request, environment, `SyncMeta` with identity and version |
| `store/` | Reading and writing TOML, walking the collection tree, moving and ordering, local UI state |
| `exec/` | The `ProtocolExecutor` trait and the HTTP implementation, building a request with variables substituted |
| `interpolate/` | The `{{variable}}` resolver: the collection environment wins over the global one |
| `import/` | Reading somebody else's format (OpenAPI today) into a plan the store writes out |
| `secrets/` | The `SecretStore` trait and its local-file implementation |
| `i18n/` | Every sentence the core shows the user, in English and Russian |
| `commands/` | The Tauri command layer — the only path by which the interface changes anything |

The key architectural rule: **the frontend never writes files**; every change goes through `store`. That is exactly where the future sync engine attaches, touching neither the domain nor the components.

The frontend is split into `stores` (reactive state), `api/client.ts` (a typed wrapper over the commands), `components`, `ui` (dialogs, errors, shared utilities) and `i18n` (the string catalogues). The types in `bindings/types.ts` mirror the Rust domain structs down to the field names, because those same structs serialize both to the files and to IPC.

## Roadmap

- [x] Incognito requests — a quick request without creating it in a workspace
- [ ] Importing collections from other formats:
    - [x] OpenAPI (3.x and Swagger 2.0)
    - [ ] Insomnia
    - [ ] Postman
    - [ ] Hoppscotch
- [ ] More protocols:
    - [ ] SSE
    - [ ] WebSocket
    - [ ] GraphQL
    - [ ] gRPC
    - [ ] Raw TCP
- [ ] Settings:
    - [x] What opens on start-up — the last workspace or the welcome screen
    - [x] Turning TLS verification on and off
    - [ ] Adding your own trusted certificates
    - [x] Connect, read and total timeouts
    - [x] Interface language
    - [ ] Proxy for requests
- [ ] Cookie management
- [ ] Markdown documentation for requests and collections inside the app
- [ ] Request scripts
- [ ] Writing tests
- [ ] Self-hosted sync server between devices
- [ ] Response history (the store is already shaped as a list of records; the cap is set to one)
- [ ] Keeping secrets in the system keychain instead of a local file
- [ ] (?) Mobile clients — would need a storage abstraction over the iOS and Android sandboxes

## License

[Apache License 2.0](LICENSE).

The project is free to use, modify and include in closed products, commercial
ones included. The license carries an explicit patent grant from contributors
and protection against patent litigation. The name "LokkiAPI" and the logo are
not covered by it — see `NOTICE` and section 6 of the license.
