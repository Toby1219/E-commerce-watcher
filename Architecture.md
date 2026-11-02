# Architecture

Verified against the source code (not by running it). Items marked *(inferred)* were deduced from structure. The endpoint reference is [api/README.md](api/README.md).

## 1. Runtime components

| Component | Tech | Role |
|-----------|------|------|
| **API** (`api/`) | Rust, actix-web 4, SeaORM 2, utoipa 6 | public HTTP API, auth, caching, background workers |
| **Embedder / scrape trigger** (`services/crawlers/api.py`) | Python, FastAPI, sentence-transformers | turns text into 768-dim vectors; launches scrapers |
| **Crawlers** (`services/crawlers/crawlers/`) | Scrapy (+ scrapy-playwright), Tortoise ORM | scrape products and FX rates into Postgres |
| **PostgreSQL + pgvector** | | system of record; HNSW index on `products.embedding` |
| **Redis** | | JSON response cache with TTLs; rate-limit storage |

Both the Rust crate and the Python Tortoise models use the **same Postgres database**. Rust migrations own the schema; the Python models mirror `products` and `currencies` and never create tables in the crawl path. The API reaches the Python service at `PY_API_HOST` (no trailing slash).

## 2. Key flows

### 2.1 Product lookup with on-demand scraping

1. `GET /api/products?p_n=<term>` runs `ILIKE %term%` on `products.product_name` (paginated).
2. If **zero** rows match, the handler fires `POST {PY_API_HOST}/run-scrapy {"text": "<term>"}` without waiting and tells the client to check back in a minute. The reply is parsed as the embedder response type, which does not fit `{"status": ...}`, so the result is always discarded (harmless, but it also hides real errors).
3. The FastAPI service lowercases and strips the term, then:
   - skips it if the same term ran within the last **1 hour** (in-memory `last_run`),
   - skips it if already in flight,
   - otherwise runs `python main.py --search=<term>` as a subprocess (max **2** concurrent, **3000 s** timeout).
4. `Spider001Spider` (Jumia NG) crawls search pages 1-4, keeps listings whose names contain **all** search words, opens each product page and yields an item (decimal price, raw price text, currency symbol `₦`).
5. `TortoiseDBPipeline` upserts by `product_url` (insert, or update price/details/reviews/specs/score).
6. New rows have `embedding = NULL` until the backfill worker fills them (2.3).

> **Status:** steps 1-4 work as described. Step 5 fails for every new product because the pipeline never supplies the NOT NULL price fields, so nothing is saved ([issue #1](docs/KNOWN_ISSUES.md)). `/api/search` triggers the same scrape when it finds no hits.

### 2.2 Semantic search

1. `GET /api/search?query=...&limit=N` embeds the query via `POST {PY_API_HOST}/text-embedder` (503 if the Python service is unreachable).
2. One SQL statement takes the **`max(limit, 200)` nearest** rows by cosine distance (`embedding <=> $1`, `embedding IS NOT NULL`), served by the HNSW index.
3. In the same statement those candidates are **filtered by name**: the query is lowercased and split on non-alphanumeric characters (stopwords `a an the for of and or in with to` dropped) and turned into a Postgres regex (`~*`, lookaheads, `\y` word boundaries), so a product is kept only if **every** query word appears as a whole word in `product_name`. Because only alphanumerics reach the regex, user input cannot inject pattern syntax. Results are ordered by distance and cut to `limit`. Distance is not a cutoff.
4. If nothing remains, the API requests a scrape for the query (as in 2.1) and returns an empty list with a "No close matches yet" message. Only non-empty results are cached (120 s).
5. On a cache miss, the query is saved to the caller's history before it is embedded.

Only `product_name` is embedded. The name filter only sees the 200 nearest candidates, so a matching product ranked lower than that is missed ([issue #10](docs/KNOWN_ISSUES.md)).

### 2.3 Embedding backfill (background)

`spawn_embedding_backfill` loops forever: select up to 64 products with `embedding IS NULL`, embed their names in one `/text-embedder` call, `UPDATE` each row, repeat until none are left. Sleeps **30 s** when idle, **60 s** after an error.

### 2.4 Authentication and tokens

```
register ──► validate (email format, username >= 3, password >= 8) ──► user_account row (Argon2 hash)
login    ──► verify password ──► insert a new `tokens` row
             ├─ access token : JWT (HS256, SECRET_KEY), 20 min, stored in tokens.token
             ├─ refresh token: 64 random alphanumerics; only its SHA-256 is stored (tokens.hash_token)
             └─ both set as cookies (HttpOnly, Secure, SameSite=None, 1 day) AND returned in the body
protected request ──► token from `access_token` cookie, else `Authorization: Bearer`
             ──► verify signature/expiry ──► tokens row with that exact JWT must exist and not be revoked
refresh  ──► token from cookie `refresh_token`, else header `X-Refresh-Token`, else JSON body
             ──► lookup by hash; reject if revoked or expired
             ──► mark that row revoked, insert a NEW row (new JWT + new refresh hash, expires 1 day from now)
             ──► set the new cookies; the JSON body currently has token/refresh_token swapped (issue #3)
logout   ──► delete the tokens row matching the presented access token
```

Every access token must also exist in the DB, so logout takes effect immediately (unlike a stateless JWT). Each login adds a row (one session per login). Refresh revokes the old row and creates a new one, so a refresh extends the session by another day. Logout deletes only the row matching the presented access token. A refresh token that was already rotated is simply rejected (no token-family revocation).

### 2.5 History for anonymous and signed-in users

`OptionalUser` extracts claims if a valid token is present; otherwise the request is anonymous and an `anon_id` UUID cookie is read or created (HttpOnly, Secure, SameSite=None, 1 day). History rows belong to either `user_id` or `anon_user`. Identical phrases are not stored twice.

### 2.6 Currency conversion

Rates are stored per currency relative to the base (USD).
- `OTB` (others to base): `amount / rate(from)`
- `OTO` (others to others): `amount * rate(to) / rate(from)`

Results are cached 120 s.

### 2.7 Currency refresh (background)

`run_curency_spider` (`api/src/tasks/other_tasks.rs`) ticks at startup, then every **24 h**, calling `GET {PY_API_HOST}/update-currency`. The FastAPI handler starts `scrapy crawl curency_api`; the spider downloads USD rates from `CURRENCY_API_URL`, and `CurrencyItemsPipeline` upserts them into `currencies`. If the call to the Python service fails, the worker retries after 60 s; on success it waits 24 h.

> **Status:** the subprocess command (`python -m scrapy crawl curency_api`) and the worker look correct in the code. I could not run them, so verify end to end after you rotate the API key ([issue #2](docs/KNOWN_ISSUES.md)).

## 3. Background workers (spawned in `main.rs`)

| Worker | Interval | Action |
|--------|----------|--------|
| Token cleanup | 150 s | delete `tokens` where `expired_at < now` |
| History cleanup | 150 s | delete **anonymous** history (`user_id IS NULL`) older than 72 h |
| Embedding backfill | 30 s idle / 60 s on error | fill `products.embedding` (2.3) |
| Currency refresh | 24 h idle / 60 s on error | ask the Python service to refresh FX rates (2.7) |

Product re-scrapes are not scheduled; they only happen through the on-demand trigger. `api/src/utils/tasks.rs` is a comment-only stub and is not compiled.

## 4. Cross-cutting behaviour

**Middleware order** (outermost first): `NormalizePath::trim` → `Logger` → CORS → route scopes.

**Route layout:** `/auth` (public) with nested `/auth/user` (protected); `/api` (public) with nested `/api/user` (protected). The governor limiter wraps the outer scopes; the JWT middleware wraps the protected ones.

**Rate limiting:** `actix-governor`, per peer IP: 3 req/s, burst 4, configured separately for `/auth` and `/api`.

**CORS:** origins are `CORS_ORIGIN`, `http://localhost:5173`, `http://127.0.0.1:5173` *(a Vite frontend that lives outside this repo)*. Methods GET/POST/PUT/DELETE/OPTIONS, allowed headers `Authorization`, `Accept`, `Content-Type` only (so a browser cannot send `X-Refresh-Token`, issue #6), credentials allowed, 1 h preflight cache.

**Caching:** Redis keys are `api:v1:<key>:<client ip>` with JSON values. TTLs: health 60 s, most reads 120 s, anonymous history 2 s. Keys include the sort type (`/api/products`), the lowercased query (`/api/search`) and the caller's identity (`/api/health`, `/api/search`). A cache read failure degrades to a DB read.

**Responses and errors:** every response uses the envelope `{status_code, message, body}`.

| `ApiError` variant | Status | Message |
|--------------------|--------|---------|
| `Unauthorized` | 401 | `Invalid credentials` |
| `JwtErr` | 401 | `Token Expired` or `Invalid Token` |
| `NotFound(msg)` | 404 | `msg` |
| `JsonError` | 400 | `Invalid JSON payload` |
| `ValidationError(msg)` | 400 | validator text |
| `DatabaseError` | 500 | `Internal server error (DB Error)` |
| `SeverError` | 500 | `Internal server error: {Server Error}` |
| `ServiceUnavailable(msg)` | 503 | `msg` (embedder unreachable) |

Unknown routes return the same envelope with 404 and `No route for <METHOD> <path>`.

**Config:** `AppEnvConfig::from_env()` panics (`expect`) on missing required variables at startup, including `PY_API_HOST`.

**Calls to the Python service:** `make_request<T>(body, path, host, method)` in `utils_funcs.rs` is a generic `get`/`post` helper that deserializes the reply into `T` and logs the whole reply at `info` (including 768-float vectors, issue #5). It builds a new `reqwest::Client` per call and sets no timeout.

**API docs:** utoipa generates OpenAPI; Swagger UI is at `/swagger-ui/`. Security schemes: `bearer_auth` (HTTP bearer JWT) and `refresh_token` (API key in the `refresh_token` cookie, used by `/auth/refresh`).

## 5. Data model

```
user_account 1───* tokens        (FK user_id, cascade)
user_account 1───* bookmarks *───1 products
user_account 1───* history       (user_id nullable; anon_user for visitors)
currencies   (standalone)
```

| Table | Columns |
|-------|---------|
| `user_account` | `id` bigint PK, `email` unique, `username`, `password_hash`, `created_at`, `joined`, `orders_count` nullable |
| `products` | `id` PK, `product_url`, `product_name`, `product_price` decimal, `product_price_text`, `product_currency`, `product_brand_name?`, `product_category?`, `product_image?`, `product_sku?`, `product_details` jsonb, `product_reviews` jsonb, `product_specifications` jsonb, `score` bigint, `created_at`, `embedding` vector(768) nullable |
| `history` | `id` PK, `phrase`, `user_id?` FK, `anon_user?` (indexed, not unique), `created_at` |
| `bookmarks` | `id` PK, `product_id` FK, `user_id` FK, `created_at` (FKs cascade) |
| `tokens` | `id` PK, `token` (JWT), `hash_token` (SHA-256 of refresh token), `revoked`, `user_id` FK, `expired_at`, `created_at` |
| `currencies` | `id` PK, `base_currency`, `currency`, `rates` decimal, `time_last_update_utc`, `time_next_update_utc` |

JSON shapes produced by the Jumia spider:

- `product_details`: `{"summary": str, "details": [{"data": {key: value}}]}`
- `product_specifications`: `{<features heading>: [str], <specs heading>: [str]}`
- `product_reviews`: `{"total_stars": "x/5", "total_reviews": str, "user_reviews": [{stars, heading, text, date, username}]}`

### Migrations (in order)

| # | Name | What it does |
|---|------|--------------|
| 1 | `create_table` | creates `user_account`, `products`, `history`, `bookmarks`, `tokens`, `currencies` from the SeaORM entities |
| 2 | `create_update_product_table` | adds `products.embedding` if missing and casts it to `vector(768)` |
| 3 | `add_products_embedding_hnsw_index` | `CREATE INDEX products_embedding_hnsw_idx ... USING hnsw (embedding vector_cosine_ops) WITH (m = 16, ef_construction = 64)` |

Install the `vector` extension before migration 1: that migration creates `products` from the entity, which includes the vector column, and the extension is only created in migration 2. There are exactly 3 migrations (`api/migration/src/lib.rs`).

## 6. Code map

### Rust API (`api/src`)

| Path | Responsibility |
|------|----------------|
| `main.rs` | load env, connect Postgres + Redis, spawn workers, build the app (CORS, logger, routes, Swagger, 404 fallback) |
| `routes/auth.rs` | `/auth/*`: register, login, refresh, update, logout |
| `routes/views.rs` | `/api/*`: health, products, search, history, bookmarks, currency, hit, order, me |
| `middleware/authentication.rs` | JWT gate for `/auth/user/*` and `/api/user/*`; puts claims in request extensions |
| `middleware/limiter.rs` | governor configs for the two scopes |
| `middleware/redis_cache.rs` | `CacheReqResp`: JSON get/set with TTL, key builder |
| `schemas/request_sch.rs` | request DTOs with validators, `SortValues`, currency conversion logic |
| `schemas/response_sch.rs` | `ApiResponse`, `ApiError`, product/user/bookmark/history DTOs |
| `schemas/others.rs` | `AppState`, `AppEnvConfig`, `Claims`, `OptionalUser`/`AuthenticatedUser` extractors, OpenAPI doc, embedder payload types |
| `utils/jwt.rs` | issue/verify JWT, refresh-token generate + hash |
| `utils/pwdhash.rs` | Argon2 hash/verify |
| `utils/utils_funcs.rs` | identity/IP helpers, token extraction, token rotation, DB lookups, history saving, `make_request` |
| `utils/text_embedder.rs` | `embed_one`, `embed_many`, `backfill_product_embeddings` |
| `utils/constants.rs` | auth cookie builder, governor type alias |
| `tasks/cleanup_task.rs` | token and anonymous-history cleanup workers |
| `tasks/embed_task.rs` | embedding backfill worker |
| `tasks/other_tasks.rs` | 24 h currency refresh worker |

### Python (`services/crawlers`)

| Path | Responsibility |
|------|----------------|
| `api.py` | FastAPI: `POST /text-embedder`, `POST /run-scrapy`, `GET /update-currency` |
| `main.py` | CLI: run `Spider001Spider` for `--search` |
| `crawlers/spiders/spider001.py` | Jumia NG product spider (`crawl001`) |
| `crawlers/spiders/curency_api.py` | exchangerate-api spider (`curency_api`) |
| `crawlers/spiders/spider002.py` | experimental eBay spider via Playwright; stores nothing yet |
| `crawlers/pipelines.py` | `TortoiseDBPipeline` (products), `CurrencyItemsPipeline` (rates) |
| `crawlers/db/` | Tortoise config and models |
| `crawlers/settings.py` | Scrapy settings |
| `crawlers/middlewares.py`, `items.py` | unmodified Scrapy templates |

## 7. Design decisions and trade-offs

| Decision | Why | Trade-off |
|----------|-----|-----------|
| Separate Python sidecar | Embedding model and Scrapy are Python-native | extra network hop; the API returns 503 if the sidecar is down |
| Local embedding model (no external API) | no per-call cost, works offline, 768-dim BGE ran fine on CPU | slower cold start; CPU-bound under load |
| pgvector instead of a dedicated vector DB | one database to run and back up; Qdrant did not fit the dev machine | less tuning than a dedicated engine |
| Scrape on a search miss | builds the catalog lazily from real searches instead of crawling everything | first searcher sees nothing; needs the scrape trigger to work (issue #1) |
| Store the JWT in the DB | instant server-side revocation on logout | one DB read per protected request (mitigated by Redis if you cache lookups) |
| Name filter on the 200 nearest candidates | dense embeddings of short brand queries (such as "hp") match poorly, so word matching keeps unrelated products out | can miss matches ranked below 200 (issue #10) |
| Current user via request extensions, not `AppState` | per-request data must not live in shared mutable state | needs the middleware to run before handlers |
| Rust migrations own the schema | single source of truth | Python models must be kept in sync by hand |