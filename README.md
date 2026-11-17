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
| Docker / CI | Working |

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

Full detail: [Architecture.md](Architecture.md) · Endpoint reference: [api/README.md](api/README.md) · Crawler service: [services/README.md](services/README.md)

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
scrapy crawl currency_api                                    # load FX rates now (spider name is misspelled "currency_api")
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
## Docker
Run the whole stack (Postgres + pgvector, Redis, Python service, Rust API) with Docker Compose.

### Requirements

- Docker Engine and the Compose plugin (`docker compose version` should work)

### 1) Create the env files
```bash
# copy the API template (create it if missing)
cp api/.env.example api/.env   

# copy the crawler template
cp services/crawlers/.env.example services/crawlers/.env  

# copy the root file env for docker use
cp ./.env.example ./.env
```
Inside Compose, services talk to each other by **service name**, not `localhost`:

| File | Variable | Docker value |
|------|----------|--------------|
| `api/.env` | `BIND_ADDRESS` | `0.0.0.0` (`127.0.0.1` would be unreachable from your host) |
| `api/.env` | `DATABASE_URL` | `postgres://<user>:<pass>@<db-service>:5432/e_watchdb` |
| `api/.env` | `REDIS_URL` | `redis://<redis-service>:6379` |
| `api/.env` | `PY_API_HOST` | `http://<python-service>:8000` (no trailing slash) |
| `services/crawlers/.env` | `DB_HOST` | `<db-service>` |
| `services/crawlers/.env` | `DB_PORT` | `5432` (the container port, not the host-mapped `5433`) |
| `services/crawlers/.env` | `DB_NAME` | `e_watchdb` (same DB the API uses) |
| `./.env`| `POSTGRES_USER` | `docker_user` (This should be in the root file for docker) |
| `./.env`| `POSTGRES_PASSWORD` | `dockeruser001` (This should be in the root file for docker) |
| `./.env`| `POSTGRES_DB` | `e_watchdb` (This should be in the root file for docker) |

### 2) Build and start

```bash
# build images and start every service in the background
docker compose up -d --build   

# check that all services are running/healthy
docker compose ps              

# follow the Rust API logs (Ctrl+C stops following, not the service)
docker compose logs -f api     
```

### 3) Prepare the database (first run only)

```bash
# enable pgvector; the first migration creates a vector column, so this must run before migrations
docker compose exec <db-service> psql -U <user> -d e_watchdb -c "CREATE EXTENSION IF NOT EXISTS vector;"

# load exchange rates so currency conversion works
docker compose exec -w /app/crawlers <python-service> scrapy crawl currency_api 
```

### 4) Use it

- Frontend UI: `http://localhost/`
- Swagger UI: `http://localhost:<PORT_NUMBER>/swagger-ui/index.html`
- Health check: `curl http://localhost:<PORT_NUMBER>/api/health`

### Common commands

```bash
 # stop and remove containers (data volumes are kept)
docker compose down           

# also delete volumes, which wipes the database
docker compose down -v         

# force a clean rebuild of one image
docker compose build --no-cache <service>   

 # restart one service after changing its .env
docker compose restart api    
```

### Notes
- **Currency API in crawler:** register an account at [exchangerate-api](https://www.exchangerate-api.com) to get an API key, so Scrapy can fetch currency data successfully. Put the key in your crawler `.env` as `CURENCY_API_URL=https://v6.exchangerate-api.com/v6/<API_KEY>/latest/USD`.  
- **Embedding model:** `api.py` forces offline mode, so `BAAI/bge-base-en-v1.5` must already be inside the Python image (or mounted as a volume). The first build is large and slow because of PyTorch and the model download.
- **Data persistence:** Postgres data lives in a named volume, so `docker compose down` keeps it and `down -v` deletes it.
- **Secrets:** never commit `.env` files. Only commit the `.env.example` templates.
- **Ports:** change the host side of a mapping (`"8181:8181"`) if the port is taken on your machine.
