# API Reference

## Conventions

**Base URL**: `http://<BIND_ADDRESS>:<PORT_NUMBER>`. A trailing slash is trimmed (`/api/products/` = `/api/products`).

**Rate limit**: 3 requests/second per IP, burst 4, on `/auth/*` and `/api/*`. Excess requests are rejected by `actix-governor` (HTTP 429).

**Envelope**: every response, success or error, has the same shape:

```json
{ "status_code": 200, "message": "Fetched products for laptop", "body": { } }
```

Errors have `body: null`:

```json
{ "status_code": 401, "message": "Invalid credentials", "body": null }
```

| Status | `message` | When |
|---|---|---|
| 400 | `Invalid JSON payload` | malformed JSON |
| 400 | validator text | a validation rule failed, or a required field is missing (register, update) |
| 401 | `Invalid credentials` | unknown email, unknown/revoked refresh token, user missing |
| 401 | `Incorrect password` | wrong password at login |
| 401 | `Token Expired` / `Invalid Token` | bad access token |
| 401 | `invalid or missing token` | no token on a protected route (auth middleware) |
| 404 | the message text | resource not found, bad parameter value, no route (`No route for GET /x`) |
| 429 | (governor default) | rate limit exceeded |
| 500 | `Internal server error (DB Error)` | database failure |
| 503 | `Error: ...` | the Python embedder service is unreachable (`/api/search`) |

**Authentication**: send the JWT as cookie `access_token` *or* header `Authorization: Bearer <jwt>` (cookie wins if both are present). Access tokens last 20 minutes; get a new one with `/auth/refresh`.

**Auth levels used below**

- **Public**: no token.
- **Optional**: token if you have one; otherwise you're treated as an anonymous visitor identified by the `anon_id` cookie (set on the response when new).
- **JWT**: a valid, non-revoked token is required.

## Endpoint summary

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/auth/register` | Public | create account |
| POST | `/auth/login` | Public | log in, get tokens |
| POST | `/auth/refresh` | Public (refresh token) | rotate tokens |
| POST | `/auth/user/update` | JWT | change email/username |
| POST | `/auth/user/logout` | JWT | end this session |
| GET | `/api/health` | Optional | health check |
| GET | `/api/products` | Public | paginated name search |
| GET | `/api/product` | Public | single product by id |
| GET | `/api/search` | Optional | semantic search, filtered by name |
| GET | `/api/currency-conversion` | Public | convert amounts |
| GET | `/api/history` | Optional | list search history |
| POST | `/api/history` | Optional | save a search phrase |
| POST | `/api/del/history` | Optional | delete a phrase |
| GET | `/api/hit` | Public | +1 product score used to update product in DB |
| GET | `/api/user/order` | JWT | +1 user order count, returns product URL |
| GET | `/api/user/me` | JWT | current user |
| GET | `/api/user/bookmark` | JWT | list bookmarks |
| POST | `/api/user/bookmark` | JWT | add bookmark |
| POST | `/api/user/del/bookmark` | JWT | remove bookmark |

---

## Auth

### `POST /auth/register`

Body (JSON): `email` (valid email), `username` (≥ 3 characters), `password_text` (≥ 8 characters). A failed rule returns `400` with the validator's message.

Returns `201` with `UserSchema` (no tokens). The password is stored as an Argon2 hash.

### `POST /auth/login`

Body: `email`, `password_text`.

Returns `200` with `UserSchema` including `token` (JWT) and `refresh_token`; also sets cookies `access_token` and `refresh_token` (HttpOnly, Secure, SameSite=None, 1 day). Each login creates a new session row. Wrong password → `401 "Incorrect password"`; unknown email → `401 "Invalid credentials"`.

### `POST /auth/refresh`

Security scheme: `refresh_token` (cookie). The refresh token is read from, in order: cookie `refresh_token`, header `X-Refresh-Token`, JSON body `{"refresh_token": "..."}`.

Returns `200` with `{"token": "<new jwt>", "refresh_token": "<new refresh>"}` and new cookies. The old refresh token stops working. Missing token → `404`; unknown/revoked/expired → `401`. A session's refresh window ends 1 day after login: refreshing does not extend it.

### `POST /auth/user/update` (JWT)

Body: `{"email": "...", "username": "..."}`. Both fields are required (`400` if missing); email must be valid and username ≥ 4 characters. Updates only what changed; returns `UserSchema` or the message `"No update to user information was made"`.

### `POST /auth/user/logout` (JWT)

Deletes the session row that matches the access token you sent, so that token is rejected from then on. Other sessions of the same user stay valid. Returns `200`.

---

## Products

### `GET /api/products`

| Query | Type | Default | Notes |
|---|---|---|---|
| `p_n` | string | **required** | product-name substring, case-insensitive. Empty/missing → `404 "Invalid product"` |
| `s_t` | `id` \| `price` \| `createdat` | `id` | ascending sort only; `price` sorts numerically |
| `p` | integer | `1` | page (min 1) |
| `p_p` | integer | `10` | page size (clamped to 1–100) |

Body (`ProductResponse`): `total_items`, `total_pages`, `current_page`, `next_page` (or null), `products[]` (see `ProductsSchema`).

If nothing matches, the response is still `200` with an empty list and the message *"No products for X yet. We're fetching them, check back in a minute."*, and a background scrape is requested. Non-empty results are cached for 120 s (the cache key includes the sort type).

### `GET /api/product?n=<id>`

Returns one `ProductsSchema` or `404 "Product not found"`. Cached 120 s.

### `GET /api/search`

| Query | Type | Default | Notes |
|---|---|---|---|
| `query` | string | **required** | free text; empty → `404` |
| `limit` | integer | `5` | clamped to 1–50 |

How it works:

1. The query is embedded and the `limit` nearest products (by cosine distance) are fetched.
2. Those rows are filtered: every word of the query (stopwords `a an the for of and or in with to` ignored) must appear as a whole word in the product name.
3. The remaining hits are returned nearest first:

```json
{
  "status_code": 200,
  "message": "searched [ hp laptop ]",
  "body": [ { "products": { /* ProductsSchema */ }, "distance": 0.231 } ]
}
```

- Fewer than `limit` hits (or none) is normal, because the filter only sees the `limit` nearest rows.
- With no hits the body is `[]`, the message is *"No close matches for [ q ] yet. We're fetching products, check back in a minute."* and a scrape is requested. Empty results are not cached; non-empty ones are cached for 120 s per caller.
- Only products that already have an embedding are searchable.
- Side effect: the query is saved to the caller's history.
- If the Python service is down → `503`.

### `GET /api/hit?product_id=<id>`

Increments the product's `score` by 1. No authentication. Returns `200` with an empty body. Unknown id → `404`.

### `GET /api/user/order?product_id=<id>` (JWT)

Call this when a user clicks through to the merchant. Increments the user's `orders_count` and returns `{"product_url": "..."}`.

---

## Account

### `GET /api/user/me` (JWT)

Returns `UserSchema` with the user's current stored access token in `token`.

### Bookmarks (JWT)

| Call | Input | Output |
|---|---|---|
| `GET /api/user/bookmark` | none | `BookmarkSchema` `{total, products[]}` |
| `POST /api/user/bookmark` | JSON `{"product_id": 1}` | `201` `{total, product}` |
| `POST /api/user/del/bookmark` | JSON `{"product_id": 1}` | `{total, removedId}`; `404` if not bookmarked |

### History (Optional auth)

Signed-in users get history by account; anonymous users by the `anon_id` cookie. Anonymous history older than 72 h is purged automatically.

| Call | Input | Output |
|---|---|---|
| `GET /api/history` | none | `HistorySchema` `{total, search: [{phrase, created_at}]}` |
| `POST /api/history` | JSON `{"phrase": "hp laptop"}` (required) | `201` `{total}` |
| `POST /api/del/history` | JSON `{"phrase": "hp laptop"}` | `{total, removedId, phrase}`; `404` if absent |

---

## Tools

### `GET /api/health`

Optional auth. Returns `200` with a body that names the caller:

- signed in: `{"message": "API says am good to go ...", "current user": "<email>"}`
- anonymous: `{"message": "API says am good to go ...", "Anonymous": "<anon id>"}` (and sets the `anon_id` cookie if it's new)

Cached 60 s per identity and IP.

### `GET /api/currency-conversion`

| Query | Type | Notes |
|---|---|---|
| `ammount` | decimal | amount to convert (sic) |
| `from` | string | currency code, e.g. `NGN` |
| `to` | string | target code. Required for `OTO`; defaults to `USD` |
| `converstion_type` | `OTB` \| `OTO` | `OTB` = other → base (USD); `OTO` = other → other |

[ OTB ` Others To Base`, OTO ` Others To Others` ]

Formulas (rates are units of currency per 1 USD): `OTB = amount / rate(from)`, `OTO = amount × rate(to) / rate(from)`.

Returns `{"amount", "from", "to", "converted"}`. Unknown currency or type → `404`. The `currencies` table must be populated first (`scrapy crawl curency_api`).

---

## Schemas

**UserSchema**: `id`, `username`, `email`, `orders_count`, `date_joined`, `token?`, `refresh_token?`

**ProductsSchema**: `id`, `product_url`, `product_name`, `product_price` (decimal, serialized as a string such as `"109990"`), `product_price_text` (as scraped, e.g. `"₦ 109,990"`), `product_currency` (e.g. `"₦"`), `product_brand_name?`, `product_category?`, `product_image?`, `product_sku?`, `product_details` (object), `product_reviews` (object), `product_specifications` (object), `score`, `created_at`

**ProductHit**: `{ products: ProductsSchema, distance: number }`

**BookmarkSchema**: `{ total, products: ProductsSchema[] }`

**HistorySchema**: `{ total, search: [{ phrase, created_at }] }`

**RefreshResponse**: `{ token, refresh_token }`

---

## Python service (internal)

Called by the Rust API at `PY_API_HOST`. Not meant for browsers.

| Method | Path | Body / response |
|---|---|---|
| POST | `/text-embedder` | `{"text": "str"}` or `{"text": ["str", ...]}` → `{"data": [768 floats]}` or `{"data": [[768 floats], ...]}` |
| POST | `/run-scrapy` | `{"text": "term"}` → `{"status": "..."}`: started, `recently scraped <term>` (1 h cooldown), or `already runnig` (sic) |
| GET | `/update-currency` | → `{"status": "..."}`: started or `recently scraped currency` |
