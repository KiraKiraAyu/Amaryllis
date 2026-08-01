# QuantAura

QuantAura is a self-hosted, single-operator full-stack application for AI-assisted trading, quantitative strategy experiments, and runtime trading operations. It includes a Rust/Axum backend and a Vue 3 frontend, with support for model and exchange configuration, strategy management, backtesting, runtime trading monitoring, debate-style decision workflows, and alerts.

## Tech Stack

- Backend: Axum, SeaORM, SQLite
- Frontend: Vue 3, Vite, Pinia, Vue Router, Tailwind CSS
- Package management: pnpm workspace
- Database: local SQLite with automatic migrations on startup

## Authentication

QuantAura is designed for self-hosting by a single operator. Access is guarded by one TOTP authenticator (Google Authenticator, 1Password, Microsoft Authenticator, etc.):

- **First boot**: open the web UI and you will be guided to scan a QR code with your authenticator app, then confirm with a 6-digit code. The secret is stored in the local database (`app_settings` table).
- **Daily access**: enter the current 6-digit code to unlock a session (JWT, HS256, 7-day TTL by default).
- **Rebind**: Settings → Security → Rebind Authenticator.
- **Optional env-pinning**: set `AUTH_TOTP_SECRET` (base32) to manage the secret outside the app; the in-app setup/rebind endpoints are then disabled.

## Local Setup

Prerequisites:

- Rust 1.92+
- Node.js and pnpm

Install frontend dependencies:

```bash
pnpm install
```

Prepare local environment variables:

```bash
cp .env.example .env
```

## Start Development

Start the backend and frontend together:

```bash
pnpm dev
```

Or start them separately:

```bash
pnpm dev:backend
pnpm dev:frontend
```

Default URLs:

- Frontend: `http://localhost:5173`
- Backend health check: `http://localhost:8000/api/health`

The Vite development server proxies `/api` requests to `http://localhost:8000`.

## Database

Default database configuration:

```env
DB_URL=sqlite://data/quantaura.db
```

The backend starts from the `core/` directory, so the default database file is `core/data/quantaura.db`. The backend creates the database file and runs migrations automatically on startup.

## Testing

Backend tests:

```bash
cd core
cargo test
cargo test --lib
cargo test --test integration_test
```

Frontend tests:

```bash
cd web
pnpm test
pnpm test:ui
pnpm test:coverage
```

## Common Development Commands

Backend checks and formatting:

```bash
cd core
cargo fmt
cargo check
```

Frontend build check:

```bash
cd web
pnpm run build
```

## Docker Deployment

Before starting the containers, prepare the root `.env` file:

```bash
cp .env.example .env
```

No key generation is needed. The TOTP secret and session signing secret are created on first boot and persisted in the `quantaura-data` volume alongside the database.

Start Docker:

```bash
docker compose up --build
```

The SQLite database is persisted in the Docker volume `quantaura-data`.

## Environment Variables

Development environment variables live in the repository root `.env` file:

- The backend reads environment variables first; during local runs it automatically tries to load `../.env` or `.env`.
- The frontend reads the root `.env` through `envDir` in `web/vite.config.ts`.
- Variables prefixed with `VITE_` are exposed to the frontend.

See `.env.example` for the common variables.
