
// import heroImg from './assets/hero.png'
// import reactLogo from './assets/react.svg'
// import viteLogo from './assets/vite.svg'
import './App.css'
import { Route, Routes, useLocation, Navigate } from "react-router-dom";

import Dashboard from './home/hompage';
import AboutPage from './home/about';
import Products_Page from './home/mul_single_prod';

import RegisterPage from './auth/registerPage';
import LoginPage from './auth/loginPage';
import ProfilePage from './auth/profile';
import BookmarksPage from './auth/bookmarkPage';
import { useCurrentUser } from './utils/api/user';


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


function ProtectedRoute({children}: { children: React.ReactNode }) {
  const { data: user, isLoading } = useCurrentUser();
  const location = useLocation();

  if (isLoading) return <LoadingOverlay />

  if (!user) {
    return <Navigate to={"/login"} state={{ from: location }} replace/>;
  }

  return <>{children}</>
  
}

function App() {
  return (
    <>
      <Routes>
        <Route path='/' element={<Dashboard/>} />
        <Route path='/about' element={<AboutPage/>} />
        <Route path='/product' element={<Products_Page/>} />

        <Route path='/register' element={<RegisterPage/>} />
        <Route path='/login' element={<LoginPage/>} />
        <Route path='/profile' element={<ProtectedRoute> < ProfilePage /> </ProtectedRoute>} />
        <Route path='/bookmark' element={<ProtectedRoute> < BookmarksPage /> </ProtectedRoute>} />


      </Routes>
    </>
  )

}

export default App
