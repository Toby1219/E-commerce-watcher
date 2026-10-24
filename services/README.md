# services: embedder and crawlers

Python 3.12 project managed with [uv](https://docs.astral.sh/uv/). It contains two things that share one codebase:

1. A **FastAPI app** (`crawlers/api.py`) that the Rust API calls over localhost: it embeds text and launches scrapes.
2. A **Scrapy project** (`crawlers/crawlers/`) that writes products and exchange rates into PostgreSQL.

## Setup

```bash
cd services                      # project root (pyproject.toml, uv.lock)
uv sync                          # create .venv and install locked dependencies
uv add sentence-transformers     # imported by api.py but not declared in pyproject.toml
cd crawlers                      # all commands below run from here
```

Create `services/crawlers/.env`:

| Variable | Default | Meaning |
|---|---|---|
| `DB_USER`, `DB_PASSWORD` | none | Postgres credentials |
| `DB_HOST` | `localhost` | host |
| `DB_PORT` | `5433` | port |
| `DB_NAME` | `e_watchdb` | database (same one the Rust API uses) |

The tables must already exist: create them with the Rust migrations in `api/migration`.

## FastAPI app (`api.py`)

```bash
uv run python -c "from sentence_transformers import SentenceTransformer as S; S('BAAI/bge-base-en-v1.5')"  # one-time model download, because api.py forces offline mode
uv run uvicorn api:app --port 8000                                                                          # serve on the port the Rust API hard-codes (127.0.0.1:8000)
```

`api.py` sets `HF_HUB_OFFLINE=1` before importing the model library, so the model must be cached locally before the first start.

| Endpoint | Body | Response |
|---|---|---|
| `POST /text-embedder` | `{"text": "str"}` or `{"text": ["str", ...]}` | `{"data": [768 floats]}` or `{"data": [[768 floats], ...]}` |
| `POST /run-scrapy` | `{"text": "search term"}` (string) | `{"status": "..."}` |

`/run-scrapy` statuses:

- `Started to Run Scrapy crawler in background`: a subprocess `python main.py --search=<term>` was started.
- `recently scraped <term>`: the same term ran in the last hour (cooldown 3600 s).
- `already runnig` (sic): the term is currently being scraped.

Limits: at most 2 scrapes at once, each killed after 3000 s. Cooldown and in-flight state live in process memory and reset on restart. Embedding model: `BAAI/bge-base-en-v1.5`, run in a worker thread so the event loop isn't blocked. CORS allows `http://localhost:8080` and `http://127.0.0.1:8080`.

## Spiders

Run any spider from `services/crawlers`.

| Spider (`scrapy crawl <name>`) | File | What it does | Writes to |
|---|---|---|---|
| `crawl001` | `spiders/spider001.py` | Jumia Nigeria. Requests search pages 1–10 for `search_key`, keeps listings whose name contains every search word, then scrapes each product page: price, summary, spec tables, features, reviews, brand, category, SKU, image | `products` via `TortoiseDBPipeline` |
| `curency_api` | `spiders/curency_api.py` | Fetches USD-based exchange rates from exchangerate-api.com, one item per currency | `currencies` via `CurrencyItemsPipeline` |
| `spider002` | `spiders/spider002.py` | Experiment: opens ebay.com with Playwright, searches, and yields the result count. Needs Chrome at `/usr/bin/google-chrome` and runs non-headless | nothing (no pipeline) |

```bash
uv run python main.py --search "hp laptop"   # run the Jumia spider for a term (spider001 requires a search key, so use main.py)
uv run scrapy crawl curency_api              # refresh exchange rates
```

`crawl001` can't be started with a bare `scrapy crawl` because its constructor requires `search_key`; use `main.py`, or `scrapy crawl crawl001 -a search_key="hp laptop"`.

## Pipelines (`crawlers/pipelines.py`)

- **`TortoiseDBPipeline`**: opens a Tortoise ORM pool, then `get_or_create` on `product_url`. For an existing product it overwrites price, details, reviews, specifications and `score`. Product names have `+` removed.
- **`CurrencyItemsPipeline`**: `update_or_create` on `(base_currency, currency)`.

## Database models (`crawlers/db/models.py`)

Tortoise models `Product` (table `products`) and `Currencies` (table `currencies`), mirroring the Rust entities. `Product` doesn't declare the `embedding` column; the Rust API fills it.

## Scrapy settings worth knowing (`crawlers/settings.py`)

| Setting | Value |
|---|---|
| `ROBOTSTXT_OBEY` | `False` |
| `CONCURRENT_REQUESTS_PER_DOMAIN` | `1` |
| `DOWNLOAD_DELAY` | `1` s |
| AutoThrottle | enabled; start delay 5 s, max 60 s, target concurrency 3 |
| HTTP cache | enabled, filesystem, 200 s expiry, dir `httpcache` |
| Reactor | asyncio |

Because robots.txt isn't obeyed, check the target site's terms of use before scraping at volume.

## Dependencies declared but not used in the code

`celery`, `aerich`, `patchright` (imported in `spider002.py` but unused). `middlewares.py` and `items.py` are untouched Scrapy templates.


