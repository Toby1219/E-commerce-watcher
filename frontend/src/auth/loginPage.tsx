// ============================================================================
// LoginPage.jsx
// Email/password sign-in form, using React Router for navigation instead of
// callback props. Same "Scout" design tokens as the rest of the app.
// ============================================================================

import { useState } from "react";
// ^ useState holds the form fields, the show/hide password flag, and any error message
import { apiRequest } from "../utils/lib/axio_init";
import { Link, useNavigate } from "react-router-dom";
// ^ Link renders the "Create one" navigation without a full page reload
//   useNavigate lets us redirect programmatically after a successful login

import { Eye, EyeOff, Mail, Lock } from "lucide-react";
// ^ Eye/EyeOff toggle password visibility; Mail/Lock are the input row icons

export default function LoginPage() {
  const navigate = useNavigate();

  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);


  const handleSubmit = (e: any) => {
    e.preventDefault();
    // ^ stop the browser's default full-page-reload form submission

    if (!email || !password) {
      setError("Enter both your email and password.");
      return;
    }

    setError("");
    setLoading(true);
    
    apiRequest("post", "/auth/login", {"email":email, "password_text":password}).then((resp)=>{
      if (resp.status === 200){
        navigate("/");
      }
      if (resp.status === 401 || resp.status === 404){
        let data = resp.data.body;
        setError(data.message);
      }
    })
    setLoading(false);
  };

  return (
    <div className="min-h-screen w-full bg-[#242220] text-[#F1E9D8] font-sans flex items-center justify-center px-6">
      <div className="w-full max-w-sm">

        <div className="flex items-center gap-2 justify-center mb-8">
          <div className="w-3 h-3 bg-[#FF6B4A] rotate-45" />
          <span className="font-serif text-lg tracking-wide">Scout</span>
        </div>

        <div className="bg-[#1D1B19] border border-[#3A362F] rounded-lg p-6">
          <h1 className="font-serif text-2xl mb-1">Welcome back</h1>
          <p className="text-sm text-[#8A8272] mb-6">Sign in to your account</p>

          <form onSubmit={handleSubmit}>
            <label className="block text-xs uppercase tracking-widest text-[#8A8272] mb-2">
              Email
            </label>
            <div className="flex items-center border-b-2 border-[#3A362F] focus-within:border-[#FF6B4A] pb-2 mb-5 transition-colors">
              <Mail size={16} className="text-[#8A8272] mr-3 shrink-0" />
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="you@example.com"
                className="flex-1 bg-transparent outline-none text-sm placeholder-[#6E6858] text-[#F1E9D8]"
              />
            </div>

            <label className="block text-xs uppercase tracking-widest text-[#8A8272] mb-2">
              Password
            </label>
            <div className="flex items-center border-b-2 border-[#3A362F] focus-within:border-[#FF6B4A] pb-2 mb-2 transition-colors">
              <Lock size={16} className="text-[#8A8272] mr-3 shrink-0" />
              <input
                type={showPassword ? "text" : "password"}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="••••••••"
                className="flex-1 bg-transparent outline-none text-sm placeholder-[#6E6858] text-[#F1E9D8]"
              />
              <button
                type="button"
                onClick={() => setShowPassword((prev) => !prev)}
                className="text-[#8A8272] hover:text-[#F1E9D8] ml-2"
                aria-label={showPassword ? "Hide password" : "Show password"}
              >
                {showPassword ? <EyeOff size={16} /> : <Eye size={16} />}
              </button>
            </div>

            <div className="flex justify-end mb-5">
              <button type="button" className="text-xs text-[#8A8272] hover:text-[#FF6B4A] transition-colors">
                Forgot password?
              </button>
            </div>

            {error && <p className="text-xs text-[#FF6B4A] mb-4">{error}</p>}

            {loading ? (
              <button
                disabled={loading}
                type="submit"
                className="w-full py-3 rounded-md bg-[#71ff4a] text-[#1D1B19] font-medium hover:bg-[#FF8266] transition-colors"
              >
                Loading ...
              </button>              
            ):
              <button
                type="submit"
                className="w-full py-3 rounded-md bg-[#FF6B4A] text-[#1D1B19] font-medium hover:bg-[#FF8266] transition-colors"
              >
                Sign in
              </button>
            }
            
          </form>
        </div>

        {/* -------- switch to register, via a router Link instead of a callback -------- */}
        <p className="text-center text-sm text-[#8A8272] mt-6">
          Don't have an account?{" "}
          <Link to="/register" className="text-[#FF6B4A] hover:underline">
            Create one
          </Link>
        </p>
      </div>
    </div>
  );
}