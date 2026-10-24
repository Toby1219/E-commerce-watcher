# E-commerce Watcher

A product-catalog API for e-commerce listings. A Python crawler scrapes products (currently **Jumia Nigeria**) into PostgreSQL, and a Rust (actix-web) API serves them with pagination, **semantic search** (pgvector), currency conversion, user accounts, bookmarks and search history.

## Features

- **Catalog API** – paginated, case-insensitive product-name search with sorting.
- **Semantic search** – query embedded with `BAAI/bge-base-en-v1.5` (768-dim), ranked by cosine distance in Postgres (`pgvector`).
- **On-demand scraping** – searching a term with no matches triggers a background Scrapy crawl for it.
- **Auth** – register/login, short-lived JWT access tokens (20 min) plus rotating refresh tokens; `HttpOnly cookies` *or* `Authorization: Bearer`.
- **User data** – bookmarks, search history (works for anonymous visitors via an `anon_id` cookie), order-click counter, product "hit" score.
- **Currency conversion** – rates (USD base/default) scraped from exchangerate-api and stored in the DB and it can be use to convert the product price to you preferred currency. Currency rate update every 24 hours.
- **Ops basics** – Redis response cache, per-IP rate limiting, CORS, Swagger UI, background cleanup workers and scraper.

## Repository layout

```
.
├── api/                      # Rust workspace (Cargo.toml at this level)
│   ├── src/                  # actix-web app: routes, middleware, schemas, utils, tasks
│   ├── entities/             # SeaORM entities (users, products, history, bookmark, tokens, currencies)
│   ├── migration/            # SeaORM migrations (+ CLI binary)
│   └── tests/routes_test.rs  # integration tests (need live Postgres + Redis)
└── services/                 # Python project (uv)
    └── crawlers/
        ├── api.py            # FastAPI: text embedder + "run scraper" trigger (port 8000)
        ├── main.py           # CLI entry to run the product spider
        └── crawlers/         # Scrapy project: spiders, pipelines, Tortoise ORM models
```

## How the pieces fit

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
        │ HTTP 127.0.0.1:8000                                  │
        ▼                                                      │
 ┌────────────────────────┐   subprocess   ┌─────────────────────┐
 │ FastAPI (services/…)   │───────────────►│ Scrapy spiders      │
 │ /text-embedder         │                │ (Jumia, FX rates)   │
 │ /run-scrapy            │                └─────────────────────┘
 | /update-currency       |
 └────────────────────────┘
```

Details: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md). Endpoint reference: [`docs/API.md`](docs/API.md). Crawler service: [`services/README.md`](services/README.md).

## Prerequisites

| Tool | Why |
|---|---|
| Recent stable **Rust** (edition 2024; the code uses `Duration::from_hours`) | build the API |
| **PostgreSQL** with the **pgvector** extension | data + vector search |
| **Redis** | response cache |
| **Python ≥ 3.12** and [**uv**](https://docs.astral.sh/uv/) | crawler + embedder service |

## Configuration

Both services read a `.env` file. **Do not commit real values** (see Known Issues).

**`api/.env`**

| Variable | Required | Meaning |
|---|---|---|
| `BIND_ADDRESS` | yes | interface to listen on, e.g. `127.0.0.1` |
| `PORT_NUMBER` | yes | port to listen on |
| `DATABASE_URL` | yes | Postgres URL (`postgres://user:pass@host:port/db`) |
| `REDIS_URL` | yes | Redis URL (`redis://host:port`) |
| `SECRET_KEY` | yes | HMAC secret used to sign JWTs |
| `CORS_ORIGIN` | yes | allowed browser origin (`http://localhost:5173` and `http://127.0.0.1:5173` are always allowed too) |
| `PY_API_HOST` | yes | python api host url |
| `RUST_LOG` | no | log verbosity. Set `info` to see worker logs; if unset, `env_logger` only shows errors |


**`services/crawlers/.env`**

| Variable | Default | Meaning |
|---|---|---|
| `DB_USER`, `DB_PASSWORD` | none | Postgres credentials |
| `DB_HOST` | `localhost` | Postgres host |
| `DB_PORT` | `5433` | Postgres port (note: not the usual 5432) |
| `DB_NAME` | `e_watchdb` | database name; must be the same DB the API uses |
| `CURRENCY_API_URL` | `https://v6.exchangerate-api.com` | currency api endpoint |

## Quick start

```bash
# 1) Database ---------------------------------------------------------------
createdb e_watchdb                                          # create the database (name matches the crawler default)
psql e_watchdb -c "CREATE EXTENSION IF NOT EXISTS vector;"  # enable pgvector so vector(768) columns work

# 2) Python embedder / scraper service --------------------------------------
cd services                                                 # Python project root (pyproject.toml + uv.lock)
uv sync                                                     # create .venv and install locked dependencies
uv add sentence-transformers                                # imported by api.py but missing from pyproject.toml
cd crawlers                                                 # uvicorn must run here so `api:app` and main.py resolve
uv run python -c "from sentence_transformers import SentenceTransformer as S; S('BAAI/bge-base-en-v1.5')"  # one-time model download (api.py later forces offline mode)
uv run uvicorn api:app --port 8000                          # Rust API expects this at http://127.0.0.1:8000 (hard-coded)

# 3) Rust API (new terminal) ------------------------------------------------
cd api                                                      # Cargo workspace root
cargo run -p migration -- up                                # apply SeaORM migrations using DATABASE_URL
cargo run                                                   # start API; also spawns cleanup + embedding workers

# 4) Seed data ---------------------------------------------------------------
cd ../services/crawlers                                     # back to the Scrapy project
uv run scrapy crawl curency_api                             # load exchange rates (spider name is spelled "curency_api")
uv run python main.py --search "hp laptop"                  # scrape Jumia for a term (or just search via the API)
```

Then open **Swagger UI** at `http://<BIND_ADDRESS>:<PORT_NUMBER>/swagger-ui/index.html` (OpenAPI JSON at `/api-docs/openapi.json`).


## Quick usage

```bash
BASE=http://127.0.0.1:8181                                   # replace with your BIND_ADDRESS:PORT_NUMBER

curl "$BASE/api/health"                                      # liveness check
curl "$BASE/api/products?p_n=laptop&p=1&p_p=10&s_t=id"       # paginated name search (empty result triggers a scrape)
curl "$BASE/api/search?query=hp%20laptop&limit=5"            # semantic (vector) search
curl "$BASE/api/currency-converstion?ammount=2500&from=NGN&converstion_type=OTB"  # NGN -> USD (path is spelled "converstion")

# Register, log in, and call a protected route (needs jq to extract the token)
curl -X POST "$BASE/auth/register" -H "Content-Type: application/json" \
  -d '{"email":"me@example.com","username":"me","password_text":"a-long-password"}'   # create the account
TOKEN=$(curl -s -X POST "$BASE/auth/login" -H "Content-Type: application/json" \
  -d '{"email":"me@example.com","password_text":"a-long-password"}' | jq -r .body.token)  # log in and capture the JWT
curl "$BASE/api/user/me" -H "Authorization: Bearer $TOKEN"   # protected route using the Bearer header
```

## Tests

```bash
cd api            # Cargo workspace root
cargo test        # integration tests in api/tests/routes_test.rs
```

The tests build the real app against the Postgres and Redis in `.env`, and assume seeded data (a product with id `10`, and `NGN`, `GBP` and `USD` rows in `currencies`). `test_search_success` is an empty stub.

## Current limitations

- Only one live source (Jumia NG). `spider002` (eBay) is an experiment that doesn't store anything yet.
- No Docker/compose files or CI config yet.