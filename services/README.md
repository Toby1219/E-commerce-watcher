# services: embedder and crawlers

Python 3.12 project managed with [uv](https://docs.astral.sh/uv/). It contains two things that share one codebase:

1. A **FastAPI app** (`crawlers/api.py`) that the Rust API calls over HTTP: it embeds text and launches scrapes.
2. A **Scrapy project** (`crawlers/crawlers/`) that writes products and exchange rates into PostgreSQL.

## Setup

```bash
cd services                                                            # project root (pyproject.toml, requirements.txt)
uv venv                                                                # create .venv
source .venv/bin/activate                                              # activate it (Windows: .venv\Scripts\activate)
uv pip install torch --index-url https://download.pytorch.org/whl/cpu  # CPU-only PyTorch, as in ../help.text
uv pip install -r requirements.txt                                     # pinned deps incl. sentence-transformers (this combination is untested)
cd crawlers                                                            # all commands below run from here
```

`requirements.txt` pins `torch==2.14.1+cpu`, a build that only the PyTorch CPU index provides, and `pyproject.toml` does not list `sentence-transformers` or `torch`. That's why the steps above use `uv pip install` rather than `uv sync` (a plain `uv sync` would not install them).

Create `services/crawlers/.env`:

| Variable | Default | Meaning |
|---|---|---|
| `DB_USER`, `DB_PASSWORD` | none | Postgres credentials |
| `DB_HOST` | `localhost` | host |
| `DB_PORT` | `5433` | port |
| `DB_NAME` | `e_watchdb` | database (same one the Rust API uses) |
| `CURENCY_API_URL` | none | full exchangerate-api URL for USD rates, including your API key (`https://v6.exchangerate-api.com/v6/<API_KEY>/latest/USD`). Spelled `CURENCY` in the code |
| `ORIGINS` | none | allowed CORS origin for the FastAPI app, e.g. `http://127.0.0.1:8181`. The value is passed to the CORS middleware as is |

The tables must already exist: create them with the Rust migrations in `api/migration`.

## FastAPI app (`api.py`)

```bash
python -c "from sentence_transformers import SentenceTransformer as S; S('BAAI/bge-base-en-v1.5')"  # one-time model download, because api.py forces offline mode
uvicorn api:app --port 8000                                                                          # serve on the port that PY_API_HOST in api/.env points to
```

`api.py` loads `.env` (`python-dotenv`), then sets `HF_HUB_OFFLINE=1` before importing the model library, so the model must be cached locally before the first start.

| Endpoint | Body | Response |
|---|---|---|
| `POST /text-embedder` | `{"text": "str"}` or `{"text": ["str", ...]}` | `{"data": [768 floats]}` or `{"data": [[768 floats], ...]}` |
| `POST /run-scrapy` | `{"text": "search term"}` (string) | `{"status": "..."}` |
| `GET /update-currency` | none | `{"status": "..."}` |

`/run-scrapy` statuses:

- `Started to Run Scrapy crawler in background for products`: a subprocess `python main.py --search=<term>` was started.
- `recently scraped <term>`: the same term ran in the last hour (cooldown 3600 s).
- `already runnig` (sic): the term is currently being scraped.

`/update-currency` starts the `curency_api` spider in the background (or answers `recently scraped currency`). The Rust API calls it every 24 hours.

Limits: at most 2 scrapes at once (shared by products and currency), each killed after 3000 s. Cooldown and in-flight state live in process memory and reset on restart. The embedding model is `BAAI/bge-base-en-v1.5`, run in a worker thread so the event loop isn't blocked.


## Spiders

Run any spider from `services/crawlers`.

| Spider (`scrapy crawl <name>`) | File | What it does | Writes to |
|---|---|---|---|
| `crawl001` | `spiders/spider001.py` | Jumia Nigeria. Requests search pages **1–4** for `search_key`, keeps listings whose name contains every search word, then scrapes each product page: price, summary, spec tables, features, reviews, brand, category, SKU, image | `products` via `TortoiseDBPipeline` |
| `curency_api` | `spiders/curency_api.py` | Fetches USD-based exchange rates from the URL in `CURENCY_API_URL`, one item per currency | `currencies` via `CurrencyItemsPipeline` |
| `spider002` | `spiders/spider002.py` | Experiment: opens ebay.com with Playwright, searches, and yields the result count. Needs Chrome at `/usr/bin/google-chrome` and runs non-headless | nothing (no pipeline) |

```bash
python main.py --search "hp laptop"   # run the Jumia spider for a term (--search is required)
scrapy crawl curency_api              # refresh exchange rates
```

`crawl001` can't be started with a bare `scrapy crawl` because its constructor requires `search_key`; use `main.py`, or `scrapy crawl crawl001 -a search_key="hp laptop"`.

### What the product spider yields

For each product: `product_url`, `product_name`, `product_brand`, `product_category`, `product_image`, `product_price` (a `Decimal` parsed from the price text; for a price range such as `₦ 1,000 - ₦ 2,000` it takes the part before the `-`), `product_price_text` (the raw text), `product_currency` (always `₦`), `product_details`, `product_specifications`, `product_reviews` and `score` (always `0`).

## Pipelines (`crawlers/pipelines.py`)

- **`TortoiseDBPipeline`**: opens a Tortoise ORM pool, then `get_or_create` on `product_url`. For an existing product it overwrites price, price text, currency, details, reviews, specifications and `score`. Product names have `+` removed.
- **`CurrencyItemsPipeline`**: `update_or_create` on `(base_currency, currency)`.

## Database models (`crawlers/db/models.py`)

Tortoise models `Product` (table `products`) and `Currencies` (table `currencies`), mirroring the Rust entities. `Product.product_price` is a decimal field (`max_digits=10, decimal_places=4`), and the model also has `product_price_text` and `product_currency`. `Product` doesn't declare the `embedding` column; the Rust API fills it.

## Scrapy settings worth knowing (`crawlers/settings.py`)

| Setting | Value |
|---|---|
| `CONCURRENT_REQUESTS_PER_DOMAIN` | `1` |
| `DOWNLOAD_DELAY` | `1` s |
| AutoThrottle | enabled; start delay 5 s, max 60 s, target concurrency 3 |
| HTTP cache | enabled, filesystem, 200 s expiry, dir `httpcache` |
| Reactor | asyncio |
