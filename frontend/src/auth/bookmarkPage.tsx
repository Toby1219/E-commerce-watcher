import { useState, useEffect } from "react";
import { useNavigate, Link } from "react-router-dom";
import { Bookmark, Trash2, Star, ArrowLeft, Package } from "lucide-react";
import { useBookmark } from "../utils/api/bookmark";
import { useDeleteBookmark } from "../utils/mutations/useDeleteData";

// ----------------------------------------------------------------------------
// Shape of one bookmarked product, as returned by the API.
// ----------------------------------------------------------------------------
interface BookmarkedProduct {
  id: number;          // bookmark row id — used to target the DELETE request
  productId: string;   // the product itself — used to link to /product/:id
  name: string;
  price: string;
  rating: string;
  reviewCount: string;
  productImgUrl: string;
}

export default function BookmarksPage() {
  const navigate = useNavigate();
  const { data:bookmarkData, isLoading, error } = useBookmark();
  const deleteBookmark = useDeleteBookmark();
  const [bookmarks, setBookmarks] = useState<BookmarkedProduct[] | null>(null);
  const [removingId, setRemovingId] = useState<number | null>(null);


  useEffect(() => {
    // if (!isLoading || !bookmarkData) return;
    let bookmark_data = bookmarkData?.products.map(
      (rows)=>({
        "id": rows.id,
        "productId": rows.product_sku,
        "name": rows.product_name,
        "price": rows.product_price,
        "rating": rows.product_reviews.total_stars,
        "reviewCount": rows.product_reviews.total_reviews,
        "productImgUrl": rows.product_image,
      })
    ) ?? [];
    setBookmarks(bookmark_data);
    console.log(bookmarkData?.products);
  }, [bookmarkData]);

  const handleRemove = (bookmarkId: number) => {
    console.log(bookmarkId);
    setRemovingId(bookmarkId);
    // ^ disable/fade just this card immediately, before the request even resolves
    deleteBookmark.mutate(bookmarkId);
  };

  return (
    <div className="min-h-screen w-full bg-[#242220] text-[#F1E9D8] font-sans">
      <div className="max-w-3xl mx-auto px-6 py-12">
        <Link
          to="/"
          className="inline-flex items-center gap-2 text-sm text-[#8A8272] hover:text-[#F1E9D8] mb-8 transition-colors"
        >
          <ArrowLeft size={16} /> Back to home
        </Link>

        <h1 className="font-serif text-3xl mb-1">Your bookmarks</h1>
        <p className="text-[#8A8272] text-sm mb-8">
          {bookmarks === null ? "Loading…" : `${bookmarks.length} saved products`}
        </p>

        {/* -------- loading state -------- */}
        {bookmarks === null && (
          <p className="text-sm text-[#8A8272]">Fetching your bookmarks…</p>
        )}

        {/* -------- empty state -------- */}
        {bookmarks !== null && bookmarks.length === 0 && (
          <div className="text-center py-16 border border-[#3A362F] rounded-lg">
            <Bookmark size={28} className="text-[#4A4638] mx-auto mb-3" />
            <p className="text-[#8A8272] text-sm">No bookmarks yet.</p>
            <Link to="/" className="text-sm text-[#FF6B4A] hover:underline mt-2 inline-block">
              Browse products
            </Link>
          </div>
        )}

        {/* -------- bookmark list -------- */}
        {bookmarks !== null && bookmarks.length > 0 && (
          <div className="space-y-3">
            {bookmarks.map((b) => (
              <div
                key={b.id}
                className={`flex items-center gap-4 bg-[#1D1B19] border border-[#3A362F] rounded-lg p-4 transition-opacity
                  ${removingId === b.id ? "opacity-40" : "opacity-100"}`}
                // ^ fades the card while its own delete request is in flight
              >
                {/* image placeholder, same package-icon stand-in used on the listing page */}
                <div className="w-14 h-14 shrink-0 bg-[#2E2A25] rounded-md flex items-center justify-center">
                  {b.productImgUrl ? 
                  (<img src={b.productImgUrl} alt="product image" />) 
                  : 
                  (<Package size={22} className="text-[#4A4638]" />)}
                  
                </div>

                {/* clicking the name/price area navigates to the product detail page */}
                <button
                  onClick={() => navigate(`/product/${b.productId}`)}
                  className="flex-1 text-left"
                >
                  <p className="text-sm text-[#F1E9D8] mb-1">{b.name}</p>
                  <div className="flex items-center gap-2">
                    <div className="flex items-center gap-0.5">
                        <Star
                          size={12}
                          className="fill-[#FF6B4A] text-[#FF6B4A]"
                        />
                    <span className="text-xs text-[#6E6858]">({b.rating})</span>
                    </div>
                    <span className="text-xs text-[#6E6858]">({b.reviewCount})</span>
                    <span className="text-sm text-[#FF6B4A] font-medium ml-2">{b.price}</span>
                  </div>
                </button>

                {/* remove button — disabled while this specific card is deleting */}
                <button
                  onClick={() => handleRemove(b.id)}
                  disabled={removingId === b.id}
                  className="text-[#8A8272] hover:text-[#FF6B4A] disabled:cursor-not-allowed transition-colors shrink-0"
                  aria-label={`Remove ${b.name} from bookmarks`}
                >
                  <Trash2 size={16} />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}