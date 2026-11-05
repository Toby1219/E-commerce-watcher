import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { Mail, Calendar, MapPin, Pencil, Check, Search, Bookmark, Package } from "lucide-react";
import { formatDistanceToNow, format } from "date-fns";
import { useCurrentUser } from "../utils/api/user";
import { useBookmark } from "../utils/api/bookmark";
import { useHistory } from "../utils/api/history";
import { useUpdateUser } from "../utils/mutations/useUpdateData";

// ----------------------------------------------------------------------------
// Seed data — stands in for a real user record fetched from an API/auth session.
// ----------------------------------------------------------------------------
interface USER{
  name: string,
  email: string,
  location: string,
  joined: Date,
  stats: {
    searches: number,
    bookmarks: number,
    orders: number,
  },
};

export default function ProfilePage() {
  const {data:currentUser, isLoading, error} = useCurrentUser();
  const {data: bookmark} = useBookmark();
  const {data: history} = useHistory();
  const updateUser = useUpdateUser();

  const [user, setUser] = useState<USER | null>(null);
  
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [location, setLocation] = useState("");

  const navigate = useNavigate();

  if (error) {
    navigate("/login"); 
    return;
  };

  useEffect(()=>{
    if (!currentUser) return;
    let loaded_data: USER = {
      name:currentUser.username,
      email:currentUser.email,
      location: "NG",
      joined: new Date(currentUser.date_joined),
      stats:{
        searches:history?.total ?? 0,
        bookmarks:bookmark?.total ?? 0,
        orders:currentUser.orders_count ?? 0
      }
    } 
    setUser(loaded_data);
    setName(loaded_data.name);
    setEmail(loaded_data.email);
    setLocation(loaded_data.location);
  }, [currentUser]);

  const handleToggleEdit = () => {
    setEditing((prev) => !prev);
    if (editing){
      updateUser.mutate({"email":email, "username":name});
      if (updateUser.isSuccess){
        let data = currentUser;
        if (!data) return;
        setUser((prev) => prev ? {
          ...prev,
          name: data.username,
          email: data.email,
        } : prev);
        setName(data.username);
        setEmail(data.email);
      }
    }
  };

  if (isLoading || !user) {
    return (
      <div className="min-h-screen w-full bg-[#242220] text-[#F1E9D8] font-sans flex items-center justify-center">
        <p className="text-sm text-[#8A8272]">Loading profile…</p>
      </div>
    );
  }

  return (
    // min-h-screen + bg-[#242220] here is what was missing: each router page
    // now needs to set its own full-page background instead of relying on
    // App.jsx's wrapper div, since React Router renders this standalone.
    <div className="min-h-screen w-full bg-[#242220] text-[#F1E9D8] font-sans">
      <div className="max-w-3xl mx-auto px-6 py-12">
        {/* page heading */}
        <h1 className="font-serif text-3xl mb-8">Your profile</h1>

      {/* -------- info card -------- */}
      <div className="bg-[#1D1B19] border border-[#3A362F] rounded-lg p-6 mb-8">
        <div className="flex items-start justify-between mb-6">
          <div className="flex items-center gap-4">
            {/* avatar placeholder: circle with the user's initial */}
            <div className="w-16 h-16 rounded-full bg-[#2E2A25] border border-[#3A362F] flex items-center justify-center font-serif text-xl text-[#FF6B4A]">
              {user?.name.charAt(0).toUpperCase()}
              {/* ^ first letter of the current name, stays in sync if the name is edited */}
            </div>

            <div>
              {editing ? (
                // edit mode: name becomes a plain text input
                <input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="bg-[#2E2A25] border border-[#3A362F] rounded-md px-2 py-1 text-lg font-serif text-[#F1E9D8] outline-none focus:border-[#FF6B4A]"
                />
              ) : (
                // read mode: name rendered as plain text
                <p className="font-serif text-lg">{name}</p>
              )}
              <p className="text-sm text-[#6E6858]">Member since {formatDistanceToNow(user ? user.joined : "", {"addSuffix":true})}</p>
            </div>
          </div>

          {/* edit/save toggle button — icon and label swap depending on mode */}
          <button
            onClick={handleToggleEdit}
            className="flex items-center gap-2 px-3 py-2 rounded-md text-sm border border-[#3A362F] text-[#B8AF9E] hover:border-[#FF6B4A] hover:text-[#FF6B4A] transition-colors"
          >
            {editing ? <Check size={14} /> : <Pencil size={14} />}
            {editing ? "Save" : "Edit"}
          </button>
        </div>

        {/* -------- contact/location rows -------- */}
        <div className="space-y-3">
          <div className="flex items-center gap-3 text-sm">
            <Mail size={16} className="text-[#8A8272]" />
            <span className="text-[#B8AF9E]">{email}</span>
            {/* email left read-only on purpose — usually changing it needs a verification flow */}
          </div>

          <div className="flex items-center gap-3 text-sm">
            <MapPin size={16} className="text-[#8A8272]" />
            {editing ? (
              <input
                value={location}
                onChange={(e) => setLocation(e.target.value)}
                className="bg-[#2E2A25] border border-[#3A362F] rounded-md px-2 py-1 text-sm text-[#F1E9D8] outline-none focus:border-[#FF6B4A]"
              />
            ) : (
              <span className="text-[#B8AF9E]">{location}</span>
            )}
          </div>

          <div className="flex items-center gap-3 text-sm">
            <Calendar size={16} className="text-[#8A8272]" />
            <span className="text-[#B8AF9E]">Joined {format(user ? user.joined : "", "dd MMM yy")}</span>
          </div>
        </div>
      </div>

      {/* -------- stats row -------- */}
      <div className="grid grid-cols-3 gap-4 mb-8">
        {/* each stat is a small card: icon, number, label — same shape repeated 3x */}
        <div className="bg-[#1D1B19] border border-[#3A362F] rounded-lg p-4 text-center">
          <Search size={18} className="text-[#FF6B4A] mx-auto mb-2" />
          <p className="text-xl font-serif">{user?.stats.searches}</p>
          <p className="text-xs text-[#8A8272]">Searches</p>
        </div>

        <div className="bg-[#1D1B19] border border-[#3A362F] rounded-lg p-4 text-center">
          <Bookmark size={18} className="text-[#FF6B4A] mx-auto mb-2" />
          <p className="text-xl font-serif">{user?.stats.bookmarks}</p>
          <p className="text-xs text-[#8A8272]">Bookmarks</p>
        </div>

        <div className="bg-[#1D1B19] border border-[#3A362F] rounded-lg p-4 text-center">
          <Package size={18} className="text-[#FF6B4A] mx-auto mb-2" />
          <p className="text-xl font-serif">{user?.stats.orders}</p>
          <p className="text-xs text-[#8A8272]">Orders</p>
        </div>
      </div>

      {/* -------- preferences section -------- */}
      <div className="bg-[#1D1B19] border border-[#3A362F] rounded-lg p-6">
        <h2 className="font-serif text-lg mb-4">Preferences</h2>

        <div className="flex items-center justify-between py-3 border-b border-[#3A362F]">
          <span className="text-sm text-[#B8AF9E]">Email me about price drops</span>
          {/* simple static toggle-look pill; wire to real state if this becomes interactive */}
          <div className="w-10 h-5 bg-[#FF6B4A] rounded-full relative">
            <div className="w-4 h-4 bg-[#1D1B19] rounded-full absolute top-0.5 right-0.5" />
          </div>
        </div>

        <div className="flex items-center justify-between py-3">
          <span className="text-sm text-[#B8AF9E]">Save search history</span>
          <div className="w-10 h-5 bg-[#FF6B4A] rounded-full relative">
            <div className="w-4 h-4 bg-[#1D1B19] rounded-full absolute top-0.5 right-0.5" />
          </div>
        </div>
      </div>
      </div>
      {/* closes the min-h-screen background wrapper added above */}
    </div>
  );
}