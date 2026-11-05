// ============================================================================
// AboutPage.jsx
// A static informational page: mission statement, how the product works,
// and a contact line. No state needed — this page is pure presentation.
// Same "Scout" design system as the rest of the app (graphite bg #242220,
// cream text #F1E9D8, coral accent #FF6B4A, serif headings + sans body).
// ============================================================================

// import React from "react";
// ^ only React itself is needed since this page has no state or effects

import { Link } from "react-router-dom";
// ^ Link swaps routes without a full page reload — used for the "back home" link below

import { Search, Bookmark, Bell, Mail, ArrowLeft } from "lucide-react";
// ^ icons for the "how it works" steps, the contact row, and the back link

// ----------------------------------------------------------------------------
// Static content — pulled out as data so the "how it works" section can be
// mapped over instead of copy-pasting the same block three times.
// ----------------------------------------------------------------------------
const STEPS = [
  {
    icon: Search,
    title: "Search once",
    text: "Type what you're after and Scout pulls matching products together in one place.",
  },
  {
    icon: Bookmark,
    title: "Save what you like",
    text: "Bookmark products to compare later instead of losing them in fifty open tabs.",
  },
  {
    icon: Bell,
    title: "Get notified",
    text: "We watch prices on your bookmarks and let you know when they drop.",
  },
];

export default function AboutPage() {
  return (
    // min-h-screen + bg-[#242220] set the full-page background directly on
    // this route (React Router mounts each page standalone, so it can't
    // rely on a shared wrapper the way the old view-state App.jsx did).
    <div className="min-h-screen w-full bg-[#242220] text-[#F1E9D8] font-sans">
      <div className="max-w-3xl mx-auto px-6 py-16">

        {/* back-to-home link, since this page now has its own route/URL */}
        <Link
          to="/"
          className="inline-flex items-center gap-2 text-sm text-[#8A8272] hover:text-[#F1E9D8] mb-10 transition-colors"
        >
          <ArrowLeft size={16} /> Back to home
        </Link>

        {/* -------- hero / mission statement -------- */}
        <div className="mb-16">
          <div className="w-3 h-3 bg-[#FF6B4A] rotate-45 mb-6" />
          {/* same diamond brand mark used in the nav, reused here for consistency */}

          <h1 className="font-serif text-3xl mb-4">About Scout</h1>

          <p className="text-[#B8AF9E] leading-relaxed">
            Scout is a search-first shopping tool. Instead of digging through five
            different stores to compare one product, you search once, see the
            options side by side, and bookmark what's worth a second look — all
            without the clutter of ads dressed up as results.
          </p>
        </div>

        {/* -------- how it works -------- */}
        <div className="mb-16">
          <h2 className="font-serif text-xl mb-8">How it works</h2>

          <div className="space-y-8">
            {STEPS.map((step, i) => {
              const Icon = step.icon;
              // ^ pull the icon component out of the data object so it can be
              //   rendered as JSX below (JSX needs a capitalized variable, not a plain object key)

              return (
                <div key={step.title} className="flex gap-4">
                  {/* numbered/iconed marker on the left of each step */}
                  <div className="shrink-0 w-10 h-10 rounded-full bg-[#1D1B19] border border-[#3A362F] flex items-center justify-center">
                    <Icon size={18} className="text-[#FF6B4A]" />
                  </div>

                  <div>
                    <h3 className="text-[#F1E9D8] mb-1">
                      {i + 1}. {step.title}
                      {/* i + 1 turns the zero-based array index into a human-friendly step number */}
                    </h3>
                    <p className="text-sm text-[#8A8272] leading-relaxed">{step.text}</p>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* -------- values -------- */}
        <div className="mb-16 border-t border-[#3A362F] pt-10">
          <h2 className="font-serif text-xl mb-4">What we care about</h2>
          <ul className="space-y-2 text-sm text-[#B8AF9E]">
            <li>— No sponsored placements pretending to be organic results.</li>
            <li>— Every product links back to where it's actually sold.</li>
            <li>— Your search history stays yours; delete it anytime from your profile.</li>
          </ul>
        </div>

        {/* -------- contact -------- */}
        <div className="border-t border-[#3A362F] pt-10">
          <h2 className="font-serif text-xl mb-4">Get in touch</h2>
          <a
            href="mailto:hello@scout.app"
            className="inline-flex items-center gap-2 text-sm text-[#B8AF9E] hover:text-[#FF6B4A] transition-colors"
          >
            <Mail size={16} />
            hello@scout.app
          </a>
        </div>
      </div>
    </div>
  );
}