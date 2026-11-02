# E-commerce Watcher

A product-search backend for e-commerce listings. A **Python/Scrapy** crawler collects products (currently **Jumia Nigeria**) into **PostgreSQL**, and a **Rust (actix-web)** API serves them with **semantic search (pgvector)**, currency conversion, user accounts, bookmarks and search history.

> **Why it's built this way:** the embedding model and Scrapy are Python-native, while the public API is Rust for speed and type safety. The two talk over HTTP and share one Postgres database.

<!-- TODO: add a screenshot or GIF of Swagger UI / the React frontend here -->

## Highlights

- **Hybrid search**: query embedded with `BAAI/bge-base-en-v1.5` (768-dim), ranked by cosine distance in `pgvector` (HNSW index), then filtered so every query word appears in the product name.
- **Lazy catalog building**: a search with no matches triggers a background Scrapy crawl for that term, so the catalog grows from real user searches.
- **Auth done properly**: Argon2 password hashing, 20-min JWT access tokens stored server-side (so logout revokes instantly), refresh tokens stored only as SHA-256 hashes, cookies (`HttpOnly`) or `Authorization: Bearer`.
- **Anonymous + signed-in history**: visitors get an `anon_id` cookie; history is cleaned up by a background worker after 72 h.
- **Production-style plumbing**: Redis response cache, per-IP rate limiting (`actix-governor`), CORS, OpenAPI/Swagger UI (`utoipa`), four background workers, one `{status_code, message, body}` envelope for every response and error.
- **Currency conversion**: USD-based rates scraped from exchangerate-api and stored in the DB.

## Status

| Area | State |
|------|-------|
| Product search (`/api/products`), semantic search (`/api/search`) | Working |
| Auth, bookmarks, history, currency conversion | Working |
| Embedding backfill worker | Working |
| Scrape trigger (API → FastAPI → Scrapy) | Fires correctly |
| Automatic FX refresh (24 h worker) | Working |
| Tests | 23 integration tests |
| Docker / CI | Not yet |

## Tech stack

| Layer | Tech |
|-------|------|
| API | Rust, actix-web 4, SeaORM 2, utoipa 6, actix-governor |
| Embedder + scrape trigger | Python 3.12, FastAPI, sentence-transformers |
| Crawlers | Scrapy, scrapy-playwright, Tortoise ORM |
| Data | PostgreSQL + pgvector, Redis |
| Tooling | uv, SeaORM migrations |

## Architecture at a glance

```
 Browser / client
       │  HTTP (cookies or Bearer JWT)
       ▼
 ┌───────────────┐   SQL    ┌──────────────────────────┐
 │  Rust API     │◄────────►│ PostgreSQL + pgvector    │◄──────┐
 │  (actix-web)  │          └──────────────────────────┘       │ writes products,
 │               │   cache  ┌───────┐                          │ currencies
 │               │◄────────►│ Redis │                          │
 └──────┬────────┘          └───────┘                          │
        │ HTTP  PY_API_HOST                                    │
        ▼                                                      │
 ┌────────────────────────┐   subprocess   ┌─────────────────────┐
 │ FastAPI (services/…)   │───────────────►│ Scrapy spiders      │
 │ /text-embedder         │                │ (Jumia, FX rates)   │
 │ /run-scrapy            │                └─────────────────────┘
 │ /update-currency       │
 └────────────────────────┘
```

Full detail: [Architecture.md](Architecture.md) · Endpoint reference: [api/README.md](api/README.md) · Crawler service: [services/README.md](services/README.md) · Known problems: [docs/KNOWN_ISSUES.md](docs/KNOWN_ISSUES.md)

## Repository layout

```
.
├── api/                      # Rust workspace (Cargo.toml here)
│   ├── README.md             # endpoint reference
│   ├── src/                  # routes, middleware, schemas, utils, background tasks
│   ├── entities/             # SeaORM entities
│   ├── migration/            # SeaORM migrations (+ CLI binary)
│   └── tests/routes_test.rs  # integration tests (need live Postgres + Redis)
├── services/                 # Python project (uv)
│   └── crawlers/             # FastAPI app, Scrapy project, Tortoise models
├── help.text                 # CPU-only PyTorch + sentence-transformers install notes
├── Architecture.md
└── docs/KNOWN_ISSUES.md
```

## Prerequisites

| Tool | Why |
|------|-----|
| Recent stable **Rust** (edition 2024) | build the API |
| **PostgreSQL** with **pgvector** | data + vector search |
| **Redis** | response cache |
| **Python ≥ 3.12** and [uv](https://docs.astral.sh/uv/) | crawler + embedder service |

## Configuration

Both services read a `.env` file. **Never commit real values.**

**`api/.env`**

| Variable | Required | Meaning |
|----------|----------|---------|
| `BIND_ADDRESS` | yes | interface to listen on, e.g. `127.0.0.1` |
| `PORT_NUMBER` | yes | port to listen on |
| `DATABASE_URL` | yes | `postgres://user:pass@host:port/db` |
| `REDIS_URL` | yes | `redis://host:port` |
| `SECRET_KEY` | yes | HMAC secret used to sign JWTs |
| `CORS_ORIGIN` | yes | allowed browser origin (`http://localhost:5173` and `http://127.0.0.1:5173` are always allowed) |
| `PY_API_HOST` | yes | Python service base URL, **no trailing slash**, e.g. `http://127.0.0.1:8000` |
| `RUST_LOG` | no | set `info` to see worker logs (unset = errors only) |

**`services/crawlers/.env`**

| Variable | Default | Meaning |
|----------|---------|---------|
| `DB_USER`, `DB_PASSWORD` | none | Postgres credentials |
| `DB_HOST` | `localhost` | Postgres host |
| `DB_PORT` | `5433` | Postgres port (not the usual 5432) |
| `DB_NAME` | `e_watchdb` | must be the same DB the API uses |
| `CURRENCY_API_URL` | none | exchangerate-api URL incl. your key: `https://v6.exchangerate-api.com/v6/<API_KEY>/latest/USD` |
| `ORIGINS` | none | allowed CORS origin(s) for the Python app |

## Quick start

```bash
# 1) Database ---------------------------------------------------------------
createdb e_watchdb                                          # create the database (name matches the crawler default)
psql e_watchdb -c "CREATE EXTENSION IF NOT EXISTS vector;"  # enable pgvector BEFORE migrating: migration 1 creates a vector column

# 2) Python embedder / scraper service --------------------------------------
cd services                                                 # Python project root
uv sync                                                     # install everything from uv.lock (pyproject.toml routes torch to the CPU-only index)
source .venv/bin/activate                                   # activate the venv uv created (Windows: .venv\Scripts\activate)
cd crawlers                                                 # uvicorn must run here so `api:app` resolves
python -c "from sentence_transformers import SentenceTransformer as S; S('BAAI/bge-base-en-v1.5')"  # one-time model download (api.py later forces offline mode)
uvicorn api:app --port 8000                                 # must match PY_API_HOST in api/.env

# 3) Rust API (new terminal) ------------------------------------------------
cd api                                                      # Cargo workspace root
cargo run -p migration -- up                                # apply the SeaORM migrations using DATABASE_URL
cargo run                                                   # start the API and its background workers

# 4) Seed data (new terminal) -----------------------------------------------
cd services/crawlers                                        # back to the Scrapy project
scrapy crawl curency_api                                    # load FX rates now (spider name is misspelled "curency_api")
python main.py --search "hp laptop"                         # scrape Jumia for a term
```

Then open **Swagger UI** at `http://<BIND_ADDRESS>:<PORT_NUMBER>/swagger-ui/index.html` (OpenAPI JSON at `/api-docs/openapi.json`). Auth schemes: `bearer_auth` (JWT) and `refresh_token` (cookie, used on `/auth/refresh`).

## Try it

```bash
BASE=http://127.0.0.1:8181                                   # replace with BIND_ADDRESS:PORT_NUMBER

curl "$BASE/api/health"                                      # liveness check
curl "$BASE/api/products?p_n=laptop&p=1&p_p=10&s_t=id"       # paginated name search
curl "$BASE/api/search?query=hp%20laptop&limit=5"            # semantic search, filtered by name
curl "$BASE/api/currency-conversion?ammount=2500&from=NGN&converstion_type=OTB"  # NGN -> USD (query params are still misspelled, issue #9)

curl -X POST "$BASE/auth/register" -H "Content-Type: application/json" \
  -d '{"email":"me@example.com","username":"me","password_text":"a-long-password"}'   # create an account
TOKEN=$(curl -s -X POST "$BASE/auth/login" -H "Content-Type: application/json" \
  -d '{"email":"me@example.com","password_text":"a-long-password"}' | jq -r .body.token)  # log in, capture the JWT (needs jq)
curl "$BASE/api/user/me" -H "Authorization: Bearer $TOKEN"   # call a protected route
```

## Tests

```bash
cd api            # Cargo workspace root
cargo test        # runs api/tests/routes_test.rs
```
