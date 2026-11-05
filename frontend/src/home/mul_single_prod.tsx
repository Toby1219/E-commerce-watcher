import { useState, useEffect } from "react";
import type { ReactNode } from "react";
import { useNavigate, useSearchParams, useLocation } from "react-router-dom";
import {
  Bookmark, User, Home, Star, ArrowLeft,
  ExternalLink, SlidersHorizontal, RotateCcw, LogInIcon
} from "lucide-react";
import { apiRequest } from "../utils/lib/axio_init";
import type { CurrentUser, Product, Products } from "../utils/store/types";
import { useAddBookmark } from "../utils/mutations/useAddData";
import { useCurrentUser } from "../utils/api/user";
import { useBookmark } from "../utils/api/bookmark";
// import { useBookmark } from "../utils/api/bookmark";


const MAX_PRICE_CEILING = 1000000000;

const RATING_OPTIONS = [0, 3, 4, 4.5];
// ^ 0 means "any rating" (no filtering); the rest are "X stars & up" thresholds


// ----------------------------------------------------------------------------
// Small reusable star-rating display.
// Renders `count` stars, filling them up to `rating` (rounded to nearest star).
// ----------------------------------------------------------------------------
function StarRating({ rating, size = 14 }: { rating: number; size?: number }) {
  const stars = Array.from({ length: 5 });
  // ^ always render exactly 5 star slots regardless of the actual rating

  return (
    <div className="flex items-center gap-0.5">
      {stars.map((_, i) => (
        <Star
          key={i}
          size={size}
          // filled if this star's index is below the rounded rating, else outline only
          className={i < Math.round(rating) ? "fill-[#FF6B4A] text-[#FF6B4A]" : "text-[#4A4638]"}
        />
      ))}
    </div>
  );
}

// ----------------------------------------------------------------------------
// 1. LOADING OVERLAY
// Full-screen layer shown on top of whichever page is loading.
// Reuses the "ruler tick" motif from the dashboard's search bar for brand cohesion.
// ----------------------------------------------------------------------------
function LoadingOverlay() {
  return (
    <div className="fixed inset-0 z-50 bg-[#242220] flex flex-col items-center justify-center">
      {/* fixed + inset-0 + z-50: pins this to cover the entire viewport above everything else */}

      <div className="w-3 h-3 bg-[#FF6B4A] rotate-45 animate-pulse mb-4" />
      {/* the same diamond brand mark from the nav, pulsing to signal activity */}

      <p className="text-[#8A8272] text-sm tracking-widest uppercase mb-6">Fetching</p>

      {/* animated tick row — ticks light up in a staggered wave via CSS animation delay */}
      <div className="flex gap-1">
        {Array.from({ length: 12 }).map((_, i) => (
          <span
            key={i}
            className="w-1 h-4 bg-[#3A362F] rounded-full animate-pulse"
            style={{ animationDelay: `${i * 80}ms` }}
            // ^ staggering each tick's delay by 80ms creates a left-to-right wave effect
          />
        ))}
      </div>
    </div>
  );
}

// ----------------------------------------------------------------------------
// Shared top nav — same as the dashboard, plus a "Shop" tab and a back-aware brand link.
// ----------------------------------------------------------------------------
function TopNav({ onHome, onBookmark, onProfile, onLogin, current_user }: 
  { onHome: () => void, onBookmark: () => void, onProfile: () => void, onLogin: () => void, current_user: CurrentUser | null }) {
  return (
    <header className="w-full border-b border-[#3A362F] bg-[#1D1B19]">
      <div className="max-w-6xl mx-auto px-6 h-16 flex items-center justify-between">
        <button className="flex items-center gap-2">
          {/* clicking the brand mark always returns to the listing page */}
          <div className="w-3 h-3 bg-[#FF6B4A] rotate-45" />
          <span className="font-serif text-lg tracking-wide">Scout</span>
        </button>

        <nav className="flex items-center gap-1 text-[#B8AF9E] text-sm">
          <button className="flex items-center gap-2 px-3 py-2 rounded-md hover:text-[#F1E9D8]" onClick={onHome}>
            <Home size={16} /> Home
          </button>
          <button className="flex items-center gap-2 px-3 py-2 rounded-md hover:text-[#F1E9D8]" onClick={onBookmark}>
            <Bookmark size={16} /> Bookmarks
          </button>
          { current_user ? (
            <button className="w-7 h-7 rounded-full bg-[#3A362F] flex items-center justify-center ml-2" onClick={onProfile}>
              <User size={14} />
            </button>
          ) : 
            <button className="w-7 h-7 rounded-full bg-[#3A362F] flex items-center justify-center ml-2" onClick={onLogin}>
              <LogInIcon size={14} />
            </button>
          }
          
        </nav>
      </div>
    </header>
  );
}

// ----------------------------------------------------------------------------
// 2. LISTING PAGE
// Grid of product cards. Clicking a card asks <App> to navigate to that product.
// ----------------------------------------------------------------------------
function ListingPage({ onSelectProduct, products }:{ onSelectProduct: (id: number) => void, products: Product[] }) {
  const [maxPrice, setMaxPrice] = useState(MAX_PRICE_CEILING);
  const [maxPriceCeiling, setMaxPriceCeiling] = useState(MAX_PRICE_CEILING);
  // ^ current price ceiling from the slider; starts at the top so nothing is filtered out initially

  const [minRating, setMinRating] = useState(0);
  // ^ current "X stars & up" threshold; 0 means the rating filter is off

  const filteredProducts = products.filter(
    (p) => p.price <= maxPrice && p.rating >= minRating
  );
  // ^ recalculated on every render from the two filter values above —
  //   no separate "filtered" state needed since it's fully derived from maxPrice/minRating

  const hasActiveFilters = maxPrice < MAX_PRICE_CEILING || minRating > 0;
  // ^ true only when the user has moved a filter away from its default —
  //   drives whether the "Reset filters" button/row is worth showing

  const resetFilters = () => {
    let prices: number[] = products.map((p) => p.price);
    setMaxPrice(Math.max(...prices));
    setMaxPriceCeiling(Math.max(...prices));
    setMinRating(0);
  };

  useEffect(() => {
    let prices: number[] = products.map((p) => p.price);
    setMaxPrice(Math.max(...prices));
    setMaxPriceCeiling(Math.max(...prices));
    setMinRating(0);
  }, [products]);

  return (
    <div className="max-w-6xl mx-auto px-6 py-12">
      <h1 className="font-serif text-3xl mb-1">Search results</h1>
      <p className="text-[#8A8272] text-sm mb-8">{filteredProducts.length} products found</p>

      {/* two-column layout: filter sidebar on the left, product grid on the right */}
      <div className="flex flex-col md:flex-row gap-8">

        {/* -------- FILTER SIDEBAR -------- */}
        <aside className="w-full md:w-56 shrink-0">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2 text-xs uppercase tracking-widest text-[#8A8272]">
              <SlidersHorizontal size={14} />
              Filters
            </div>

            {/* reset only appears once a filter has actually been changed */}
            {hasActiveFilters && (
              <button
                onClick={resetFilters}
                className="flex items-center gap-1 text-xs text-[#8A8272] hover:text-[#FF6B4A] transition-colors"
              >
                <RotateCcw size={12} /> Reset
              </button>
            )}
          </div>

          {/* -------- price range filter -------- */}
          <div className="bg-[#1D1B19] border border-[#3A362F] rounded-lg p-4 mb-4">
            <div className="flex items-center justify-between mb-3">
              <span className="text-sm text-[#F1E9D8]">Price</span>
              <span className="text-sm text-[#FF6B4A]">
                {/* shows "Up to $200" only when unfiltered, otherwise the exact ceiling */}
                Up to {maxPrice.toLocaleString("en-NG", {style:"currency", currency:"NGN", maximumFractionDigits:0})}
              </span>
            </div>

            <input
              type="range"
              min={0}
              max={maxPriceCeiling}
              step={10}
              value={maxPrice}
              onChange={(e) => setMaxPrice(Number(e.target.value))}
              // ^ range inputs always return their value as a string, so Number(...)
              //   converts it back before it's compared against p.price above
              className="w-full accent-[#FF6B4A]"
              // ^ accent-* is a Tailwind utility that recolors native range/checkbox controls
              //   to match the brand's coral accent instead of the browser default blue
            />

            <div className="flex justify-between text-xs text-[#6E6858] mt-1">
              <span>$0</span>
              <span>{maxPriceCeiling.toLocaleString("en-NG", {style:"currency", currency:"NGN", maximumFractionDigits:0})}</span>
            </div>
          </div>

          {/* -------- rating filter -------- */}
          <div className="bg-[#1D1B19] border border-[#3A362F] rounded-lg p-4">
            <span className="text-sm text-[#F1E9D8] block mb-3">Rating</span>

            <div className="flex flex-col gap-1">
              {RATING_OPTIONS.map((threshold) => (
                <button
                  key={threshold}
                  onClick={() => setMinRating(threshold)}
                  // ^ clicking a row selects that threshold as the new minRating
                  className={`flex items-center gap-2 px-2 py-1.5 rounded-md text-sm transition-colors text-left
                    ${minRating === threshold
                      ? "bg-[#2E2A25] text-[#FF6B4A]"        // this threshold is the active one
                      : "text-[#B8AF9E] hover:bg-[#2E2A25]"}`}  // inactive: neutral, highlights on hover
                >
                  {threshold === 0 ? (
                    <span>Any rating</span>
                  ) : (
                    <>
                      <StarRating rating={threshold} size={12} />
                      <span>& up</span>
                    </>
                  )}
                </button>
              ))}
            </div>
          </div>
        </aside>

        {/* -------- PRODUCT GRID -------- */}
        <div className="flex-1">
          {filteredProducts.length === 0 ? (
            // empty state: shown when the current filter combination matches nothing
            <div className="text-center py-20 text-[#8A8272]">
              <p className="mb-3">No products match these filters.</p>
              <button
                onClick={resetFilters}
                className="text-sm text-[#FF6B4A] hover:underline"
              >
                Clear filters
              </button>
            </div>
          ) : (
            // responsive grid: 1 column on mobile, 2 on small screens, 3 from md up
            <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-5">
              {filteredProducts.map((p) => (
                <button
                  key={p.id}
                  onClick={() => onSelectProduct(p.id)}
                  className="text-left bg-[#1D1B19] border border-[#3A362F] rounded-lg p-4 hover:border-[#FF6B4A] transition-colors"
                >
                  {/* image placeholder — a bordered box with a package icon stands in for a real photo */}
                  <div className="w-full aspect-square bg-[#2E2A25] rounded-md flex items-center justify-center mb-3">
                    {/* <Package size={32} className="text-[#4A4638]" /> */}
                    <img src={p.image_url} alt={p.name} />
                  </div>

                  <h3 className="text-sm text-[#F1E9D8] mb-1 line-clamp-2">{p.name}</h3>

                  <div className="flex items-center gap-2 mb-1">
                    <StarRating rating={p.rating} />
                    <span className="text-xs text-[#6E6858]">({p.reviewCount})</span>
                  </div>

                  <span className="text-[#FF6B4A] font-medium">{p.price.toLocaleString("en-NG", {style:"currency", currency:"NGN", maximumFractionDigits:0})}</span>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------------------
// 3. SINGLE PRODUCT PAGE
// Shows full details, a size/option selector (only for clothing/shoes),
// reviews + rating breakdown, and a button out to the original listing.
// ----------------------------------------------------------------------------
// titled block with a top rule, used for each section below the fold
function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mt-12 border-t border-[#3A362F] pt-8">
      <h2 className="font-serif text-xl mb-5">{title}</h2>
      {children}
    </section>
  );
}

// two-column list with the brand diamond as the bullet
function BulletList({ items }: { items: string[] }) {
  return (
    <ul className="grid grid-cols-1 sm:grid-cols-2 gap-x-8 gap-y-2">
      {items.map((line, i) => (
        <li key={i} className="flex gap-3 text-sm text-[#B8AF9E] leading-relaxed">
          <span className="mt-2 w-1.5 h-1.5 bg-[#FF6B4A] rotate-45 shrink-0" />
          {line}
        </li>
      ))}
    </ul>
  );
}

function ProductPage({ productId, products, loading, currentUser,  onBack }: {
  productId: number; products: Product[]; loading: boolean; onBack: () => void; currentUser: CurrentUser | null;
}) {
  const addBookmark = useAddBookmark();
  const { data:bookmark } = useBookmark(); 
  const product = products.find((p) => p.id === productId);
  const [selectedSize, setSelectedSize] = useState<string | null>(null);

  // const navigate = useNavigate();
  
  const isBookmarked = bookmark?.products.some((p) => p.id === productId) ?? false;

  // derive the label from state instead of storing it, so it can never go out of sync
  const bagText = isBookmarked
    ? "Added to bag"                       // already bookmarked (reload, or just added)
    : addBookmark.isPending
      ? "Adding…"                          // request in flight
      : addBookmark.isError
        ? "Couldn't add, try again"        // request failed and was rolled back
        : "Add to bag"; 

  useEffect(()=>{
    apiRequest("post", `/api/hit`, {product_id:productId}).catch((err) => {
      console.log(err);
    })


  },[products]);
  
  const bookmarkHandler = (product: Product) => {
    if (!currentUser){
      console.log("HI")
      return
    }
    addBookmark.mutate(product as unknown as Products);

    console.log("productId:", productId, typeof productId); // what the page thinks the id is
    console.log("bookmark ids:", bookmark?.products.map((p) => [p.id, typeof p.id, p.product_sku])); // what the cache holds
  };

  const handleOriginalUrl = () => {
    if (!product?.originalUrl) { return }
    apiRequest("post", "/api/user/order", {product_id:productId}).then(
      (resp) => {
        if (resp.status === 200) {
          console.log(product.originalUrl)
          // if (resp.data.body.product_url === product.originalUrl) {
          //   window.open(encodeURIComponent(product.originalUrl), "_blank", "noopener,noreferrer");

          // }
        }
      }
    ).catch((err)=>{ console.log(err); return})
  };

  // products may not have arrived yet (e.g. page opened directly on a product URL)
  if (!product) {
    return (
      <p className="text-center py-20 text-[#8A8272]">
        {loading ? "Loading product…" : "Product not found."}
      </p>
    );
  }

  const hasSizes = product.sizes.length > 0;
  const specGroups = Object.entries(product.specifications);      // only non-empty groups (see toSpecs)
  const crumbs = product.category.split("/").filter(Boolean);     // "A/B/C" -> ["A","B","C"]
  const price = product.price.toLocaleString("en-NG", { style: "currency", currency: "NGN", maximumFractionDigits: 0 });

  return (
    <div className="max-w-5xl mx-auto px-6 py-12">
      <button onClick={onBack} className="flex items-center gap-2 text-sm text-[#8A8272] hover:text-[#F1E9D8] mb-8">
        <ArrowLeft size={16} /> Back to results
      </button>

      {/* -------- top: image + buy box -------- */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-10">
        <div className="w-full aspect-square bg-[#1D1B19] border border-[#3A362F] rounded-lg flex items-center justify-center">
          <img className="p-4 max-h-full object-contain" src={product.image_url} alt={product.name} />
        </div>

        <div>
          {crumbs.length > 0 && <p className="text-xs text-[#6E6858] mb-3">{crumbs.join(" › ")}</p>}
          <h1 className="font-serif text-2xl mb-2">{product.name}</h1>

          <p className="text-xs text-[#8A8272] mb-3">
            {product.brand && <>Brand: <span className="text-[#B8AF9E]">{product.brand}</span></>}
            {product.sku && <span className="ml-3">SKU: <span className="text-[#B8AF9E]">{product.sku}</span></span>}
          </p>

          <div className="flex items-center gap-2 mb-4">
            {product.rating > 0 ? (
              <>
                <StarRating rating={product.rating} size={16} />
                <span className="text-sm text-[#F1E9D8]">{product.rating}</span>
                <span className="text-sm text-[#6E6858]">· {product.reviewCount} reviews</span>
              </>
            ) : (
              <span className="text-sm text-[#6E6858]">No ratings yet</span>
            )}
          </div>

          <p className="text-2xl text-[#FF6B4A] font-medium mb-6">{price}</p>

          {hasSizes && (
            <div className="mb-6">
              <p className="text-xs uppercase tracking-widest text-[#8A8272] mb-2">Select size</p>
              <div className="flex flex-wrap gap-2">
                {product.sizes.map((size) => (
                  <button
                    key={size}
                    onClick={() => setSelectedSize(size)}
                    className={`px-3 py-1.5 rounded-md border text-sm transition-colors
                      ${selectedSize === size
                        ? "border-[#FF6B4A] text-[#FF6B4A] bg-[#2E2A25]"
                        : "border-[#3A362F] text-[#B8AF9E] hover:border-[#8A8272]"}`}
                  >
                    {size}
                  </button>
                ))}
              </div>
            </div>
          )}

          <button
            onClick={() => bookmarkHandler(product)}
            disabled={(hasSizes && !selectedSize) || isBookmarked || addBookmark.isPending}
            className="w-full sm:w-auto px-6 py-3 rounded-md bg-[#FF6B4A] text-[#1D1B19] font-medium
                       disabled:bg-[#3A362F] disabled:text-[#6E6858] disabled:cursor-not-allowed
                       hover:bg-[#FF8266] transition-colors mb-3 block"
          >
            {bagText}
          </button>

          <a
            onClick={handleOriginalUrl}
            href={product.originalUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center justify-center sm:justify-start gap-2 w-full sm:w-auto px-6 py-3 rounded-md
                       border border-[#3A362F] text-sm text-[#B8AF9E] hover:border-[#FF6B4A] hover:text-[#FF6B4A] transition-colors"
          >
            <ExternalLink size={16} /> View original listing
          </a>
        </div>
      </div>

      {/* -------- summary (hidden when the API sends none) -------- */}
      {product.description.summary && (
        <Section title="Description">
          <p className="text-sm text-[#B8AF9E] leading-relaxed max-w-3xl">{product.description.summary}</p>
        </Section>
      )}

      {/* -------- product_details.details -------- */}
      {product.description.details.length > 0 && (
        <Section title="Details">
          <BulletList items={product.description.details} />
        </Section>
      )}

      {/* -------- one section per non-empty spec group: Key Features, What's in the box, ... -------- */}
      {specGroups.map(([group, items]) => (
        <Section key={group} title={group}>
          <BulletList items={items} />
        </Section>
      ))}

      {/* -------- reviews -------- */}
      <Section title={`Reviews (${product.reviewCount})`}>
        {product.reviews.length === 0 ? (
          <p className="text-sm text-[#6E6858]">No reviews yet.</p>
        ) : (
          <div className="space-y-5 max-w-2xl">
            {product.reviews.map((r, index) => (
              <div key={r.id} className="border-b border-[#3A362F] pb-5">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-[11px] text-[#F1E9D8] font-bold">User {index + 1}</span>
                  <StarRating rating={r.rating} size={13} />
                </div>
                <br />
                {r.heading && <p className="text-sm text-[#F1E9D8] font-bold mb-1">{r.heading}</p>}
                <br />
                <p className="text-sm text-[#B8AF9E] leading-relaxed">{r.text}</p>
                {r.date && <p className="text-xs text-[#6E6858] mt-2">{r.date}</p>}
              </div>
            ))}
          </div>
        )}
      </Section>
    </div>
  );
}

// ----------------------------------------------------------------------------
// Products — owns the "which page + are we loading" state and switches between
// LoadingOverlay / ListingPage / ProductPage accordingly.
// ----------------------------------------------------------------------------
export default function Products_Page() {
  const { data: currentUser } = useCurrentUser();
  const [searchParams, setSearchParams] = useSearchParams();
  const navigate = useNavigate();
  const location = useLocation();

  const phrase = searchParams.get("s") ?? "";          // read live from the URL, never stale
  const selectedProductId = searchParams.has("p") ? Number(searchParams.get("p")) : null; // null = listing view

  const [products, setProducts] = useState<Product[]>([]);
  const [loading, setLoading] = useState(true);


  // collapse runs of whitespace and trim
  const clean = (s?: string | null): string => (s ?? "").replace(/\s+/g, " ").trim();

  // first number in a string: "₦ 1,492,900" -> 1492900
  const parseNumber = (s?: string | null): number => {
    const m = s?.match(/\d[\d,]*(?:\.\d+)?/);
    return m ? parseFloat(m[0].replace(/,/g, "")) : 0;
  };

  // "4.5/5" -> 4.5, but "None/5" -> 0 (the old parseNumber returned 5 for "None/5")
  const parseRating = (s?: string | null): number => {
    const m = s?.match(/^\s*(\d+(?:\.\d+)?)\s*\/\s*5/);
    return m ? parseFloat(m[1]) : 0;
  };

  // turns a spec item (string or {label: value}) into one display line
  const toLine = (x: unknown): string =>
    typeof x === "string"
      ? clean(x)
      : x && typeof x === "object"
        ? Object.entries(x).map(([k, v]) => `${clean(k)}: ${clean(String(v ?? ""))}`).join(", ")
        : "";

  // product_details.details is [{data: {key: value}}], with some null values -> flat list of lines
  const toDetailLines = (details: any): string[] =>
    Array.isArray(details)
      ? details.flatMap((d) =>
          Object.entries(d?.data ?? {})
            .filter(([, v]) => typeof v === "string" && clean(v as string)) // drops the "null": null rows
            .map(([k, v]) =>
              clean(k) === clean(v as string) ? clean(v as string) : `${clean(k)}: ${clean(v as string)}`
            )
        )
      : [];

  // product_specifications -> only the groups that actually have items
  const toSpecs = (specs: any): Record<string, string[]> =>
    Object.fromEntries(
      Object.entries(specs ?? {})
        .map(([group, items]): [string, string[]] => [
          group,
          (Array.isArray(items) ? items : []).map(toLine).filter(Boolean),
        ])
        .filter(([, items]) => items.length > 0)
    );

  // one API row -> Product, keeping every field the product page needs
  const toProduct = (row: any): Product => {
    const p = row.products;
    const rev = p.product_reviews ?? {};
    const userReviews: any[] = Array.isArray(rev.user_reviews) ? rev.user_reviews : [];
    return {
      id: p.id,
      name: clean(p.product_name),
      brand: clean(p.product_brand_name),
      category: p.product_category ?? "",
      sku: p.product_sku ?? "",
      price: parseNumber(p.product_price),
      rating: parseRating(rev.total_stars),
      reviewCount: parseNumber(rev.total_reviews) || userReviews.length,
      sizes: [],
      image_url: p.product_image ?? "",
      originalUrl: p.product_url ?? "",
      description: {
        summary: clean(p.product_details?.summary),
        details: toDetailLines(p.product_details?.details),
      },
      specifications: toSpecs(p.product_specifications),
      reviews: userReviews.map((r, i) => ({
        id: i + 1,
        author: clean(r.username).replace(/^by\s+/i, ""), // API sends "by Salisu Opeyemi"
        rating: parseNumber(r.stars),                      // "1 out of 5" -> 1
        date: clean(r.date),
        heading: clean(r.heading),
        text: clean(r.text),
      })),
    };
  };

  // listing -> product: pushes a NEW history entry, so browser Back returns to ?s=phrase
  const openProduct = (id: number) =>
    setSearchParams({ s: phrase, p: String(id) }, { state: { fromListing: true } });

  // "Back to results" button: step back in history if we came from the listing,
  // otherwise (page opened directly on a product URL) just drop `p`
  const backToResults = () => {
    if (location.state?.fromListing) navigate(-1);
    else setSearchParams({ s: phrase }, { replace: true });
  };

  // fetch depends ONLY on `phrase`, so opening/closing a product never refetches
  useEffect(() => {
    let cancelled = false;                              // ignore stale responses if phrase changes mid-request
    setLoading(true);
    apiRequest("get", `api/search?query=${encodeURIComponent(phrase)}&limit=30`)
      .then((resp) => {
        if (!cancelled && resp.status === 200) setProducts(resp.data.body.map(toProduct));
      })
      .catch((error) => console.error("Search request failed:", error))
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [phrase]);

  useEffect(() => { window.scrollTo(0, 0); }, [selectedProductId]); // top of page on every switch
  
  return (
    <div className="min-h-screen w-full bg-[#242220] text-[#F1E9D8] font-sans relative">
      {loading && <LoadingOverlay />}
      <TopNav onHome={() => navigate("/")} 
        onBookmark={() => navigate("/bookmark")} 
        onProfile={() => navigate("/profile")} 
        onLogin={() => navigate("/login")} 
        current_user={currentUser || null} 
      />

      {selectedProductId === null ? (
        <ListingPage onSelectProduct={openProduct} products={products} />
      ) : (
        <ProductPage productId={selectedProductId} 
          products={products} loading={loading} 
          onBack={backToResults} currentUser={currentUser || null} 
        />
      )}
    </div>
  );
}