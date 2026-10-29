# Architecture

## 1. Runtime components

| Component | Tech | Role |
|---|---|---|
| **API** (`api/`) | Rust, actix-web 4, SeaORM 2, utoipa 6 | public HTTP API, auth, caching, background workers |
| **Embedder / scrape trigger** (`services/crawlers/api.py`) | Python, FastAPI, sentence-transformers | turns text into 768-dim vectors; launches scrapers |
| **Crawlers** (`services/crawlers/crawlers/`) | Scrapy (+ scrapy-playwright), Tortoise ORM | scrape products + FX rates into Postgres |
| **PostgreSQL + pgvector** | | system of record; vector similarity search (HNSW index on `products.embedding`) |
| **Redis** | | JSON response cache with TTLs, rate-limit storage |

Both the Rust crate and the Python Tortoise models talk to the **same Postgres database**. The Rust migrations own the schema; the Python models mirror `products` and `currencies` and never create tables in the crawl path.

The Rust API reaches the Python service at `PY_API_HOST` (an environment variable, no trailing slash).

## 2. Key flows

### 2.1 Product lookup with on-demand scraping

1. `GET /api/products?p_n=<term>` runs an `ILIKE %term%` query on `products.product_name` (paginated).
2. If **zero** rows match, the handler fire-and-forgets `POST {PY_API_HOST}/run-scrapy {"text": "<term>"}` and tells the client to check back in a minute. Errors from this call are discarded.
3. The FastAPI service lowercases/strips the term, then:
   - skips it if the same term ran within the last **1 hour** (in-memory `last_run`),
   - skips it if already in flight,
   - otherwise starts `python main.py --search=<term>` as a subprocess in a background task (max **2** concurrent scrapes, **3000 s** timeout).
4. `Spider001Spider` (Jumia NG) crawls search pages 1–4, keeps listings whose names contain **all** search words, opens each product page and yields an item (price parsed to a decimal, the raw price text, currency symbol `₦`).
5. `TortoiseDBPipeline` upserts by `product_url` (insert, or update price/details/reviews/specs/score).
6. New rows have `embedding = NULL` until the backfill worker (below) fills them.

> **Status:** as of the latest commit, `/run-scrapy` raises an error on every call (see `docs/KNOWN_ISSUES.md`, #1), so steps 3–6 don't currently run from the API.

### 2.2 Semantic search

1. `GET /api/search?query=...&limit=N` embeds the query through `POST {PY_API_HOST}/text-embedder` (if the Python service is unreachable the API answers **503**).
2. A raw SQL statement orders `products` by cosine distance (`embedding <=> $1`) where `embedding IS NOT NULL`, taking the **`limit` nearest** rows. The HNSW index serves this query shape.
3. The nearest rows are then **filtered by name**: the query is lowercased and split into words (the stopwords `a an the for of and or in with to` are dropped), and a product is kept only if **every** query word appears as a whole word in its `product_name`. Distance is not used as a cutoff.
4. If nothing remains, the API requests a scrape for the query (as in 2.1) and returns an empty list with the message "No close matches for [ q ] yet...". Only non-empty results are cached (120 s).
5. The query is saved to the caller's history before the search runs.

Only `product_name` is embedded (not details or reviews). Because the filter runs after the vector search on just `limit` rows, a matching product ranked lower than `limit` by meaning is missed (see Known Issues).

### 2.3 Embedding backfill (background)

`spawn_embedding_backfill` loops forever: select up to 64 products with `embedding IS NULL`, embed their names in one `/text-embedder` call, `UPDATE` each row; repeat until none are left. Sleeps **30 s** when idle, **60 s** after an error.

### 2.4 Authentication and tokens

```
register ──► validate (email format, username >= 3, password >= 8) ──► user_account row (Argon2 hash)
login    ──► verify password ──► insert a new `tokens` row
             ├─ access token : JWT (HS256, SECRET_KEY), 20 min, stored in tokens.token
             └─ refresh token: 64 random alphanumerics; only its SHA-256 is stored (tokens.hash_token)
             └─ both set as cookies (HttpOnly, Secure, SameSite=None, 1 day) AND returned in the body
protected request ──► token from `access_token` cookie, else `Authorization: Bearer`
             ──► verify signature/expiry ──► tokens row with that exact JWT must exist and not be revoked
refresh  ──► token from cookie `refresh_token`, else header `X-Refresh-Token`, else JSON body
             ──► lookup by hash; reject if revoked or expired (1 day after login)
             ──► overwrite token, hash_token and revoked=false on that same row; return the new pair
logout   ──► delete the tokens row that matches the presented access token
```

Because every access token must also exist in the DB, logout takes effect immediately (unlike a stateless JWT). Each login adds a row, so a user can have one session per login; refresh and logout only touch the row they were called with. `expired_at` is set at login and is not extended by refresh.

### 2.5 History for anonymous and signed-in users

`OptionalUser` extracts claims if a valid token is present, otherwise the request is treated as anonymous: an `anon_id` UUID cookie is read or created (HttpOnly, Secure, SameSite=None, 1 day). History rows belong to either `user_id` or `anon_user`. Identical phrases are not stored twice.

### 2.6 Currency conversion

Rates are stored per currency relative to the base (USD). `(Others To Base) OTB` = `amount / rate(from)`. `(Others To Others) OTO` = `amount * rate(to) / rate(from)`. Results are cached 120 s.

### 2.7 Currency refresh (background)

`run_curency_spider` (`api/src/tasks/other_tasks.rs`) ticks immediately at startup and then every **24 h**. Each tick calls `GET {PY_API_HOST}/update-currency`. The FastAPI handler starts `scrapy crawl curency_api` in the background; the spider downloads USD rates from the URL in `CURENCY_API_URL`, and `CurrencyItemsPipeline` upserts them into `currencies`. If the call fails, the worker logs the error and sleeps 200 s, but the next call still happens only at the next 24 h tick.

> **Status:** the subprocess command in `run_currency_scraper` is built incorrectly (see Known Issues, #2), so rates are not refreshed by this path yet. Run `scrapy crawl curency_api` by hand.

## 3. Background workers (spawned in `main.rs`)

| Worker | Interval | Action |
|---|---|---|
| Token cleanup | 150 s | delete `tokens` where `expired_at < now` |
| History cleanup | 150 s | delete **anonymous** history (`user_id IS NULL`) older than 72 h |
| Embedding backfill | 30 s idle / 60 s on error | fill `products.embedding` (see 2.3) |
| Currency refresh | 24 h idle / 60 s on error | ask the Python service to refresh FX rates (see 2.7) |

Product re-scrapes are not scheduled; they only happen through the on-demand trigger. `api/src/utils/tasks.rs` is a comment-only stub and is not compiled.

## 4. Cross-cutting behaviour

**Middleware order** (outermost first): `NormalizePath::trim` → `Logger` → CORS → route scopes.

**Rate limiting**: `actix-governor`, per peer IP: 3 req/s with burst 4, separately configured for `/auth` and `/api` (same numbers).

**CORS**: origins are `CORS_ORIGIN`, `http://localhost:5173`, `http://127.0.0.1:5173` *(inferred: a separate Vite frontend; none in this repo yet)*. Methods GET/POST/PUT/DELETE/OPTIONS, credentials allowed, 1 h preflight cache.

**Caching**: Redis keys are `api:v1:<key>:<client ip>` with JSON values. TTLs: health 60 s, most reads 120 s, anonymous history 2 s. Keys now include the sort type (`/api/products`), the lowercased query (`/api/search`) and the caller's identity (`/api/health`, `/api/search`). A cache read failure degrades to a DB read.

**Responses and errors**: every response, success or error, uses the envelope `{status_code, message, body}`. `ApiError` variants map to statuses:

| Variant | Status | Message |
|---|---|---|
| `Unauthorized` | 401 | `Invalid credentials` |
| `JwtErr` | 401 | `Token Expired` or `Invalid Token` |
| `NotFound(msg)` | 404 | `msg` |
| `JsonError` | 400 | `Invalid JSON payload` |
| `ValidationError(msg)` | 400 | validator text |
| `DatabaseError` | 500 | `Internal server error (DB Error)` |
| `SeverError` | 500 | `Internal server error: {Server Error}` |
| `ServiceUnavailable(msg)` | 503 | `msg` (used when the embedder is unreachable) |

Unknown routes return the same envelope with a 404 and `No route for <METHOD> <path>`.

**Config**: `AppEnvConfig::from_env()` panics (via `expect`) on missing required variables at startup, including `PY_API_HOST`.

**Calls to the Python service**: `make_request<T>(body, path, host, method)` in `utils_funcs.rs` is a generic helper for `get`/`post` calls; it deserializes the reply into `T` and logs it at `info` level.

**API docs**: utoipa generates the OpenAPI document; Swagger UI is mounted at `/swagger-ui/`. Registered security schemes: `bearer_auth` (HTTP bearer JWT) and `refresh_token` (API key in the `refresh_token` cookie, used by `/auth/refresh`).

## 5. Data model

```
user_account 1───* tokens        (FK user_id, cascade)
user_account 1───* bookmarks *───1 products
user_account 1───* history       (user_id nullable; anon_user for visitors)
currencies   (standalone)
```

| Table | Columns (type notes) |
|---|---|
| `user_account` | `id` bigint PK, `email` unique, `username`, `password_hash`, `created_at`, `joined`, `orders_count` nullable |
| `products` | `id` PK, `product_url`, `product_name`, `product_price` **decimal**, `product_price_text`, `product_currency`, `product_brand_name?`, `product_category?`, `product_image?`, `product_sku?`, `product_details` jsonb, `product_reviews` jsonb, `product_specifications` jsonb, `score` bigint, `created_at`, `embedding` vector(768) **nullable** |
| `history` | `id` PK, `phrase`, `user_id?` FK, `anon_user?` (indexed, not unique), `created_at` |
| `bookmarks` | `id` PK, `product_id` FK, `user_id` FK, `created_at` (FKs cascade) |
| `tokens` | `id` PK, `token` (the JWT), `hash_token` (SHA-256 of refresh token), `revoked`, `user_id` FK, `expired_at`, `created_at` |
| `currencies` | `id` PK, `base_currency`, `currency`, `rates` decimal, `time_last_update_utc`, `time_next_update_utc` |

JSON shapes produced by the Jumia spider:

- `product_details`: `{"summary": str, "details": [{"data": {key: value}}]}`
- `product_specifications`: `{<features heading>: [str], <specs heading>: [str]}`
- `product_reviews`: `{"total_stars": "x/5", "total_reviews": str, "user_reviews": [{stars, heading, text, date, username}]}`

### Migrations (in order)

| # | Name | What it does |
|---|---|---|
| 1 | `create_table` | creates `user_account`, `products`, `history`, `bookmarks`, `tokens`, `currencies` from the SeaORM entities |
| 2 | `create_update_product_table` | adds `products.embedding` if missing and casts it to `vector(768)` |
| 3 | `add_products_embedding_hnsw_index` | `CREATE INDEX products_embedding_hnsw_idx ... USING hnsw (embedding vector_cosine_ops) WITH (m = 16, ef_construction = 64)` |

The `vector` extension should be installed before migration 1, because that migration creates `products` with a vector column *(inferred from the migration order)*.

## 6. Code map

### Rust API (`api/src`)

| Path | Responsibility |
|---|---|
| `main.rs` | load env, connect Postgres + Redis, spawn workers, build the app (CORS, logger, routes, Swagger, 404 fallback) |
| `routes/auth.rs` | `/auth/*`: register, login, refresh, update, logout |
| `routes/views.rs` | `/api/*`: health, products, search, history, bookmarks, currency, hit, order, me |
| `middleware/authentication.rs` | JWT gate for `/auth/user/*` and `/api/user/*`; puts claims in request extensions |
| `middleware/limiter.rs` | governor configs for the two scopes |
| `middleware/redis_cache.rs` | `CacheReqResp`: JSON get/set with TTL, key builder |
| `schemas/request_sch.rs` | request DTOs with validators, `SortValues`, currency `Converstions` logic |
| `schemas/response_sch.rs` | `ApiResponse`, `ApiError`, product/user/bookmark/history DTOs |
| `schemas/others.rs` | `AppState`, `AppEnvConfig`, `Claims`, `OptionalUser`/`AuthenticatedUser` extractors, OpenAPI doc + security schemes, embedder payload types |
| `utils/jwt.rs` | issue/verify JWT, refresh-token generate + hash |
| `utils/pwdhash.rs` | Argon2 hash/verify |
| `utils/utils_funcs.rs` | identity/IP helpers, `bearer_or_cookie_token`, token rotation, DB lookups, history saving, generic `make_request` to the Python service |
| `utils/text_embedder.rs` | `embed_one`, `embed_many`, `backfill_product_embeddings` (both take the Python host) |
| `utils/constants.rs` | auth cookie builder, governor type alias |
| `tasks/cleanup_task.rs` | token and anonymous-history cleanup workers |
| `tasks/embed_task.rs` | embedding backfill worker |
| `tasks/other_tasks.rs` | 24-hour currency refresh worker |

### Python (`services/crawlers`)

| Path | Responsibility |
|---|---|
| `api.py` | FastAPI app: `POST /text-embedder`, `POST /run-scrapy`, `GET /update-currency` |
| `main.py` | CLI: run `Spider001Spider` for `--search` |
| `crawlers/spiders/spider001.py` | Jumia NG product spider (`crawl001`) |
| `crawlers/spiders/curency_api.py` | exchangerate-api spider (`curency_api`), URL from `CURENCY_API_URL` |
| `crawlers/spiders/spider002.py` | experimental eBay spider via Playwright (`spider002`) |
| `crawlers/pipelines.py` | `TortoiseDBPipeline` (products), `CurrencyItemsPipeline` (rates) |
| `crawlers/db/` | Tortoise config and models |
| `crawlers/settings.py` | Scrapy settings |
| `crawlers/middlewares.py`, `items.py` | unmodified Scrapy templates |

## 7. Design notes *(inferred)*

- **Why a separate Python service?** The embedding model and Scrapy are Python-native; the Rust API treats them as a sidecar over HTTP.
- **Why scrape on a miss?** It builds the catalog lazily from real user searches instead of crawling everything up front.
- **Why store the JWT in the DB?** To allow server-side revocation of access tokens.
- **Why filter semantic results by name?** Dense embeddings of short brand queries (such as "hp") match poorly, so requiring the query words in the product name keeps unrelated products out.