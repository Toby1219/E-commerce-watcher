import { useState, useEffect } from "react";
import { Bookmark, User, Clock, X, ArrowRight, LogIn, LogOut, PackageSearch } from "lucide-react";
import { formatDistanceToNow } from "date-fns";
import { useNavigate } from "react-router-dom";

import { apiRequest } from "../utils/lib/axio_init";
import { useCurrentUser } from "../utils/api/user";
import { useBookmark } from "../utils/api/bookmark";
import { useDeleteHistory } from "../utils/mutations/useDeleteData";
import { useAddHistory } from "../utils/mutations/useAddData";
import { useHistory } from "../utils/api/history";

type SEEDHISTORY={
  id:number,
  phrase:string,
  time:string
}

// ----------------------------------------------------------------------------
// Main component
// ----------------------------------------------------------------------------
export default function Dashboard() {
  // ---- state -----------------------------------------------------------
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const [history, setHistory] = useState<SEEDHISTORY[]>();
  const [bookmarks, setBookmark] = useState<number>();
  const [activeNav, setActiveNav] = useState("search");

  const { data: user } = useCurrentUser();
  const { data: bookmarkData } = useBookmark();
  const { data: histroyData, isLoading:loadingHistory, } = useHistory();
  const addHistory = useAddHistory();
  const deleteHistory = useDeleteHistory();


  useEffect(()=>{
    let search = histroyData?.search.map((rows, index)=>({
      id:index+1,
      phrase: rows.phrase,
      time: formatDistanceToNow(rows.created_at, {addSuffix:true})
    }))
    setHistory(search);
    setBookmark(bookmarkData?.total);
  }, [bookmarkData, histroyData])

  useEffect(()=>{
    console.log(history);
  },[history])


  // ---- handlers ----------------------------------------------------------
  const handleSearch = (e:any) => {
    e.preventDefault();
    const trimmed = query.trim();
    if (!trimmed) return;
    const newEntry = {
      id: Date.now(), 
      phrase: trimmed,     
      time: "just now",      
    };
    addHistory.mutate({phrase: trimmed});
    if (addHistory.isSuccess){
      setHistory((prev) => [newEntry, ...(prev ?? [])]);
    }
    setQuery("");
    navigate(`/product?s=${trimmed}`)
  };

  const handleHistoryClick = (phrase:string) => {
    setQuery(phrase);
  };

  const removeHistoryItem = (id: number, phrase:string, e: any) => {
    e.stopPropagation();
    
    setHistory((prev) => prev?.filter((item) => item.id !== id || item.phrase !== phrase));
    deleteHistory.mutate({phrase:phrase});
    console.log(histroyData);
  };

  const logOutHandler = () => {
    apiRequest("post", "/auth/user/logout").then((resp) => {
      if (resp.status === 200){ navigate("/login"); return}
    })
  }

  if (loadingHistory){ return <p className="text-sm text-[#8A8272]">Fetching your bookmarks…</p>; };
  // if (errorUser || errorHistory || errorbookmark){ navigate("/login"); return; };


  // ---- render --------------------------------------------------------------
  return (
    // Outer shell: full-height flex column, warm graphite background, cream text
    <div className="min-h-screen w-full bg-[#242220] text-[#F1E9D8] flex flex-col font-sans">

      {/* ================= TOP NAV ================= */}
      <header className="w-full border-b border-[#3A362F] bg-[#1D1B19]">
        <div className="max-w-6xl mx-auto px-6 h-16 flex items-center justify-between">

          {/* Left: brand mark */}
          <div className="flex items-center gap-2">
            {/* small square acts as a logo mark — coral accent square */}
            <div className="w-3 h-3 bg-[#FF6B4A] rotate-45" />
            <span className="font-serif text-lg tracking-wide">Scout</span>
          </div>

          {/* Right: nav actions — history / bookmarks / profile */}
          <nav className="flex items-center gap-1">
            {/* History tab */}
            <button
              onClick={() => {setActiveNav("search");}}
              // ^ clicking sets this tab as active, which drives the underline style below
              className={`flex items-center gap-2 px-3 py-2 rounded-md text-sm transition-colors
                ${activeNav === "search"
                  ? "bg-[#2E2A25] text-[#FF6B4A]"   // active state: subtle fill + accent text
                  : "text-[#B8AF9E] hover:text-[#F1E9D8]"}`}  // inactive: muted, brightens on hover
            >
              <Clock size={16} className="text-white" />
              <span>History</span>
            </button>
            {/* Bookmarks tab, with a small count badge */}
            <button
              onClick={() => {setActiveNav("bookmarks"); navigate("/bookmark")}}
              className={`flex items-center gap-2 px-3 py-2 rounded-md text-sm transition-colors
                ${activeNav === "bookmarks"
                  ? "bg-[#2E2A25] text-[#FF6B4A]"
                  : "text-[#B8AF9E] hover:text-[#F1E9D8]"}`}
            >
              <Bookmark size={16} />
              <span>Bookmarks</span>
              {/* badge only renders if there's at least one bookmark */}
              {bookmarks ? (
                bookmarks > 0 && (
                  <span className="ml-1 text-xs bg-[#FF6B4A] text-[#1D1B19] rounded-full w-5 h-5 flex items-center justify-center font-medium">
                    {bookmarks}
                  </span>
                )
              ) : 
                <span className="ml-1 text-xs bg-[#FF6B4A] text-[#1D1B19] rounded-full w-5 h-5 flex items-center justify-center font-medium" > 0 </span>
              }
            </button>

            {/* Profile tab */}
            {user ? (
            <>
              <button
                onClick={() => {setActiveNav("profile"); navigate("/profile")}}
                className={`flex items-center gap-2 px-3 py-2 rounded-md text-sm transition-colors ml-2
                  ${activeNav === "profile"
                    ? "bg-[#2E2A25] text-[#FF6B4A]"
                    : "text-[#B8AF9E] hover:text-[#F1E9D8]"}`}
              >
                {/* profile shown as a circular avatar with a User icon inside */}
                <span className="w-7 h-7 rounded-full bg-[#3A362F] flex items-center justify-center">
                  <User size={14} />
                </span>
              </button>

              <button
                onClick={() => {setActiveNav("logout"); logOutHandler()}}
                className={`flex items-center gap-2 px-3 py-2 rounded-md text-sm transition-colors ml-2
                  ${activeNav === "profile"
                    ? "bg-[#2E2A25] text-[#FF6B4A]"
                    : "text-[#B8AF9E] hover:text-[#F1E9D8]"}`}
              >
                {/* profile shown as a circular avatar with a User icon inside */}
                <span className="w-7 h-7 rounded-full bg-[#3A362F] flex items-center justify-center">
                  <LogOut size={14} />
                </span>
              </button>

            </>
            ) : (

            <button
              onClick={() => {setActiveNav("login"); navigate("/login");}}
              className={`flex items-center gap-2 px-3 py-2 rounded-md text-sm transition-colors ml-2
                ${activeNav === "profile"
                  ? "bg-[#2E2A25] text-[#FF6B4A]"
                  : "text-[#B8AF9E] hover:text-[#F1E9D8]"}`}
            >
              {/* profile shown as a circular avatar with a User icon inside */}
              <span className="w-7 h-7 rounded-full bg-[#3A362F] flex items-center justify-center">
                <LogIn size={14} />
              </span>
            </button>
            ) }
          </nav>
        </div>
      </header>

      {/* ================= BODY: sidebar + main stage ================= */}
      <div className="flex flex-1 max-w-6xl mx-auto w-full">

        {/* ---------- LEFT SIDEBAR: search history ledger ---------- */}
        <aside className="w-64 shrink-0 border-r border-[#3A362F] px-4 py-6 hidden md:block">
          {/* shrink-0 stops it from being squeezed by the flex-1 main area;
              hidden md:block hides it on very small screens and shows it from md breakpoint up */}

          <h2 className="text-xs uppercase tracking-widest text-[#8A8272] mb-4 px-2">
            Recent searches
          </h2>

          <ul className="space-y-1">
            {history?.length === 0 && (
              // empty state: shown only when the history array has no items left
              <li className="text-sm text-[#6E6858] px-2 py-3">
                No searches yet — try the box on the right.
              </li>
            )}

            {history?.map((item) => (
              <li
                key={item.id}
                // ^ React needs a stable, unique key per list item to track it efficiently
                onClick={() => handleHistoryClick(item.phrase)}
                className="group flex items-center justify-between px-2 py-2 rounded-md cursor-pointer hover:bg-[#2E2A25] transition-colors"
              >
                <div className="flex flex-col overflow-hidden">
                  {/* the phrase itself, truncated with ellipsis if too long */}
                  <span className="text-sm text-[#E7DCC5] truncate">{item.phrase}</span>
                  <span className="text-xs text-[#6E6858]">{item.time}</span>
                </div>

                {/* remove (X) button — only visible on hover via the `group` class above */}
                <button
                  onClick={(e) => removeHistoryItem(item.id, item.phrase, e)}
                  className="opacity-0 group-hover:opacity-100 text-[#8A8272] hover:text-[#FF6B4A] transition-opacity shrink-0 ml-2"
                  aria-label={`Remove ${item.phrase} from history`}
                >
                  <X size={14} />
                </button>
              </li>
            ))}
          </ul>
        </aside>

        {/* ---------- MAIN STAGE: centered search bar ---------- */}
        <main className="flex-1 flex flex-col items-center justify-center px-6 py-24">

          {/* headline above the search bar sets context for the page's single job */}
          <h1 className="font-serif text-3xl sm:text-4xl text-center mb-2">
            What are you hunting for?
          </h1>
          <p className="text-[#8A8272] text-sm mb-10 text-center">
            Search products, compare, and bookmark the ones you like.
          </p>

          {/* the search form itself — submitting calls handleSearch */}
          <form onSubmit={handleSearch} className="w-full max-w-xl">
            <div className="relative flex items-center border-b-2 border-[#FF6B4A] pb-2">
              {/* magnifying glass icon sits inside the input's left padding area */}
              <PackageSearch size={20} className="text-[#8A8272] mr-3 shrink-0" />

              <input
                type="text"
                value={query}
                // ^ controlled input: its displayed value always mirrors the `query` state
                onChange={(e) => setQuery(e.target.value)}
                // ^ every keystroke updates state, keeping React as the single source of truth
                placeholder="Try “wireless keyboard” or “desk lamp”"
                className="flex-1 bg-transparent outline-none text-lg placeholder-[#6E6858] text-[#F1E9D8]"
              />

              {/* submit arrow button, disabled (dimmed) when the input is empty */}
              <button
                type="submit"
                disabled={!query.trim()}
                className="ml-3 text-[#FF6B4A] disabled:text-[#4A4638] disabled:cursor-not-allowed transition-colors"
                aria-label="Submit search"
              >
                <ArrowRight size={20} />
              </button>
            </div>

            {/* tick-mark ruler under the input — a small decorative signature element,
                echoes a measuring/ledger motif rather than a generic shadowed search bar */}
            <div className="flex justify-between mt-1 px-1">
              {Array.from({ length: 24 }).map((_, i) => (
                // 24 tiny vertical ticks, taller every 6th tick to suggest a ruled scale
                <span
                  key={i}
                  className={`w-px bg-[#3A362F] ${i % 6 === 0 ? "h-2" : "h-1"}`}
                />
              ))}
            </div>
          </form>

          {/* quick suggestion chips built from the 3 most recent history phrases */}
          {history ? 
          (
            history?.length > 0 && (
              <div className="flex flex-wrap gap-2 justify-center mt-8 max-w-xl">
                {history?.slice(0, 3).map((item) => (
                  <button
                    key={item.id}
                    onClick={() => handleHistoryClick(item.phrase)}
                    className="text-xs px-3 py-1.5 rounded-full border border-[#3A362F] text-[#B8AF9E] hover:border-[#FF6B4A] hover:text-[#FF6B4A] transition-colors"
                  >
                    {item.phrase}
                  </button>
                ))}
              </div>
            )
          ) : <></>
          }
        </main>
      </div>
    </div>
  );
}