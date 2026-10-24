# Architecture

## 1. Runtime components

| Component | Tech | Role |
|---|---|---|
| **API** (`api/`) | Rust, actix-web 4, SeaORM 2, utoipa | public HTTP API, auth, caching, background workers |
| **Embedder / scrape trigger** (`services/crawlers/api.py`) | Python, FastAPI, sentence-transformers | turns text into 768-dim vectors; launches scrapers |
| **Crawlers** (`services/crawlers/crawlers/`) | Scrapy (+ scrapy-playwright), Tortoise ORM | scrape products + FX rates into Postgres |
| **PostgreSQL + pgvector** | | system of record; vector similarity search |
| **Redis** | | JSON response cache with TTLs |

Both the Rust crate and the Python Tortoise models talk to the **same Postgres database**. The Rust migrations own the schema; the Python models mirror `products` and `currencies` and never create tables in the crawl path.

## 2. Key flows

### 2.1 Product lookup with on-demand scraping

1. `GET /api/products?p_n=<term>` runs an `ILIKE %term%` query on `products.product_name` (paginated).
2. If **zero** rows match, the handler fire-and-forgets `POST http://127.0.0.1:8000/run-scrapy {"text": "<term>"}` and tells the client to check back in a minute.
3. The FastAPI service lowercases/strips the term, then:
   - skips it if the same term ran within the last **1 hour** (in-memory `last_run`),
   - skips it if already in flight,
   - otherwise starts `python main.py --search=<term>` as a subprocess in a background task (max **2** concurrent scrapes, **3000 s** timeout).
4. `Spider001Spider` (Jumia NG) crawls search pages 1–10, keeps listings whose names contain **all** search words, opens each product page and yields an item.
5. `TortoiseDBPipeline` upserts by `product_url` (insert, or update price/details/reviews/specs/score).
6. New rows have `embedding = NULL` until the backfill worker (below) fills them.

### 2.2 Semantic search

1. `GET /api/search?query=...&limit=N` records the query in the caller's history.
2. The API calls `POST /text-embedder` for the query vector.
3. A raw SQL statement orders `products` by cosine distance (`embedding <=> $1`) where `embedding IS NOT NULL`, returning each product plus `distance`.
4. Result cached in Redis for 120 s.

Only `product_name` is embedded (not details or reviews).

### 2.3 Embedding backfill (background)

`spawn_embedding_backfill` loops forever: select up to 64 products with `embedding IS NULL`, embed their names in one `/text-embedder` call, `UPDATE` each row; repeat until none are left. Sleeps **30 s** when idle, **60 s** after an error.

### 2.4 Authentication and tokens

```
register ──► user_account row (Argon2 hash)
login    ──► verify password ──► insert `tokens` row
             ├─ access token : JWT (HS256, SECRET_KEY), 20 min, stored in tokens.token
             └─ refresh token: 64 random alphanumerics; only its SHA-256 is stored (tokens.hash_token)
             └─ both set as cookies (HttpOnly, Secure, SameSite=None, 1 day) AND returned in the body
protected request ──► token from `access_token` cookie, else `Authorization: Bearer`
             ──► verify signature/expiry ──► tokens row with that exact JWT must exist and not be revoked
refresh  ──► token from cookie `refresh_token`, else header `X-Refresh-Token`, else JSON body
             ──► lookup by hash; reject if revoked or expired (1 day)
             ──► mark it revoked, issue new access + refresh, then overwrite whichever token row `find(user_id).one()` returns (the matched row when the user has only one; see Known Issues #9)
logout   ──► delete the user's token row
```

Because every access token must also exist in the DB, logout/revocation takes effect immediately (unlike a stateless JWT).

### 2.5 History for anonymous and signed-in users

`OptionalUser` extracts claims if a valid token is present, otherwise the request is treated as anonymous: an `anon_id` UUID cookie is read or created (HttpOnly, Secure, SameSite=None, 1 day). History rows belong to either `user_id` or `anon_user`. Identical phrases are not stored twice.

### 2.6 Currency conversion

Rates are stored per currency relative to the base (USD). `(Others To Base) OTB` = `amount / rate(from)`. `(Others To Others) OTO` = `amount * rate(to) / rate(from)`. Results are cached 120 s.

## 3. Background workers (spawned in `main.rs`)

| Worker | Interval | Action |
|---|---|---|
| Token cleanup | 150 s | delete `tokens` where `expired_at < now` |
| History cleanup | 150 s | delete **anonymous** history (`user_id IS NULL`) older than 72 h |
| Embedding backfill | 30 s idle / 60 s on error | fill `products.embedding` (see 2.3) |

Nothing in the repo schedules the currency spider or periodic re-scrapes. `api/src/utils/tasks.rs` is a comment-only stub listing such TODOs and is not compiled (it isn't declared in `utils/mod.rs`).

## 4. Cross-cutting behaviour

**Middleware order** (outermost first): `NormalizePath::trim` → `Logger` → CORS → route scopes.

**Rate limiting** – `actix-governor`, per peer IP: 5 req/s with burst 7, separately configured for `/auth` and `/api` (same numbers).

**CORS** – origins: `CORS_ORIGIN`, `http://localhost:5173`, `http://127.0.0.1:5173` *(inferred: a separate Vite frontend; none in this repo)*. Methods GET/POST/PUT/DELETE/OPTIONS, credentials allowed, 1 h preflight cache.

**Caching** – Redis keys are `api:v1:<query lowercased>:<client ip>` with JSON values. TTLs: health 60 s, most reads 120 s, anonymous history 2 s. A cache read failure degrades to a DB read.

**Errors** – success bodies use the envelope `{status_code, message, body}`. `ApiError` values are returned as a bare JSON string with the matching HTTP status. Unknown routes return `{"status_code":"404","details":"No route for <METHOD> <path>"}`.

**Config** – `AppEnvConfig::from_env()` panics (via `expect`) on missing required variables at startup.

**API docs** – utoipa generates the OpenAPI document; Swagger UI is mounted at `/swagger-ui/`.

## 5. Data model

```
user_account 1───* tokens        (FK user_id, cascade)
user_account 1───* bookmarks *───1 products
user_account 1───* history       (user_id nullable; anon_user for visitors)
currencies   (standalone)
items        (standalone demo table, not used by any route)
```

| Table | Columns (type notes) |
|---|---|
| `user_account` | `id` bigint PK, `email` unique, `username`, `password_hash`, `created_at`, `joined`, `orders_count` nullable. A later migration also adds an `anon_user` column that the entity doesn't map |
| `products` | `id` PK, `product_url`, `product_name`, `product_price` **text**, `product_brand_name?`, `product_category?`, `product_image?`, `product_sku?`, `product_details` jsonb, `product_reviews` jsonb, `product_specifications` jsonb, `score` bigint, `created_at`, `embedding` vector(768) |
| `history` | `id` PK, `phrase`, `user_id?` FK, `anon_user?` (unique + indexed), `created_at` |
| `bookmarks` | `id` PK, `product_id` FK, `user_id` FK, `created_at` (FKs cascade) |
| `tokens` | `id` PK, `token` (the JWT), `hash_token` (SHA-256 of refresh token), `revoked`, `user_id` FK, `expired_at`, `created_at` |
| `currencies` | `id` PK, `base_currency`, `currency`, `rates` decimal, `time_last_update_utc`, `time_next_update_utc` |
| `items` | `id` serial PK, `content`, `embedding` vector(768) with an HNSW cosine index (migration only) |

JSON shapes produced by the Jumia spider:

- `product_details`: `{"summary": str, "details": [{"data": {key: value}}]}`
- `product_specifications`: `{<features heading>: [str], <specs heading>: [str]}`
- `product_reviews`: `{"total_stars": "x/5", "total_reviews": str, "user_reviews": [{stars, heading, text, date, username}]}`

Migrations (in order): initial tables from the entity structs → `currencies` → `anon_user` on `user_account` → `items` demo table + `vector` extension → `products.embedding` → `anon_user` on `history`.

## 6. Code map

### Rust API (`api/src`)

| Path | Responsibility |
|---|---|
| `main.rs` | load env, connect Postgres + Redis, spawn workers, build the app (CORS, logger, routes, Swagger) |
| `routes/auth.rs` | `/auth/*`: register, login, refresh, update, logout |
| `routes/views.rs` | `/api/*`: health, products, search, history, bookmarks, currency, hit, order, me |
| `middleware/authentication.rs` | JWT gate for `/auth/user/*` and `/api/user/*`; puts claims in request extensions |
| `middleware/limiter.rs` | governor configs for the two scopes |
| `middleware/redis_cache.rs` | `CacheReqResp`: JSON get/set with TTL, key builder |
| `schemas/request_sch.rs` | request DTOs, `SortValues`, currency `Converstions` logic |
| `schemas/response_sch.rs` | `ApiResponse`, `ApiError`, product/user/bookmark/history DTOs |
| `schemas/others.rs` | `AppState`, `AppEnvConfig`, `Claims`, `OptionalUser`/`AuthenticatedUser` extractors, OpenAPI doc |
| `utils/jwt.rs` | issue/verify JWT, refresh-token generate + hash |
| `utils/pwdhash.rs` | Argon2 hash/verify |
| `utils/utils_funcs.rs` | identity/IP helpers, token rotation, DB lookups, history saving, HTTP call to the Python service |
| `utils/text_embedder.rs` | `embed_one`, `embed_many`, `backfill_product_embeddings` |
| `utils/constants.rs` | auth cookie builder, governor type alias |
| `tasks/` | background workers (cleanup, embedding backfill) |

### Python (`services/crawlers`)

| Path | Responsibility |
|---|---|
| `api.py` | FastAPI app: `/text-embedder`, `/run-scrapy` |
| `main.py` | CLI: run `Spider001Spider` for `--search` |
| `crawlers/spiders/spider001.py` | Jumia NG product spider (`crawl001`) |
| `crawlers/spiders/curency_api.py` | exchangerate-api spider (`curency_api`) |
| `crawlers/spiders/spider002.py` | experimental eBay spider via Playwright (`spider002`) |
| `crawlers/pipelines.py` | `TortoiseDBPipeline` (products), `CurrencyItemsPipeline` (rates) |
| `crawlers/db/` | Tortoise config and models |
| `crawlers/settings.py` | Scrapy settings |
| `crawlers/middlewares.py`, `items.py` | unmodified Scrapy templates |

## 7. Design notes *(inferred)*

- **Why a separate Python service?** The embedding model and Scrapy are Python-native; the Rust API treats them as a sidecar over localhost HTTP.
- **Why scrape on a miss?** It builds the catalog lazily from real user searches instead of crawling everything up front.
- **Why store the JWT in the DB?** To allow server-side revocation of access tokens.