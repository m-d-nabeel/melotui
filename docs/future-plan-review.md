# Phase 2: The Real-World Product

## Critical Engineering Review: The Feature Wishlist

### 1. What the proposal solves

Phase 1 — agreed, do it. No further discussion needed.

Everything after Phase 1 — you've listed roughly 12-15 features with no prioritization. I need to push back on the framing, not the vision.

### 2. Whether it solves it

It doesn't solve anything yet because it's a wishlist, not a plan. You said the right thing: *"thinking about different frontend is not a good idea right now."* Apply that same discipline to this list. You cannot build all of these simultaneously, and attempting to will result in 15 half-finished features instead of 3 solid ones.

### 3. What it costs

Let me sort your list by what it actually is:

| Category | Features | Reality Check |
|----------|----------|---------------|
| **Core UX (must-have)** | Multi-folder support, better search/filter, playlist CRUD polish, intuitive navigation, help screen improvements | These make or break whether anyone uses the app. ~2-4 weeks of focused work. |
| **Nice-to-have** | Better equalizer, playlist config/keybind customization, lyrics display | Real features, but the app works without them. ~1-2 weeks each. |
| **Research projects** | LLM mood filtering, music extraction, acapella extraction | These are each entire projects with their own dependency chains, model selection, inference runtime decisions, and UX challenges. LLM inference alone requires choosing between local (ggml/candle) vs API, dealing with latency, model size, etc. Acapella/stem separation requires integrating something like `demucs` — a Python ML model. These are **months** of work, not tasks on a checklist. |

### 4. My concern

> [!WARNING]
> You're conflating "features I want eventually" with "what I should build next." These are not the same thing. Listing them together creates the illusion that they're comparable in scope. They are not. LLM mood filtering is a bigger project than everything else on this list combined.

The features that will make your app *actually good to use today* are boring: multi-folder support, better search, playlist management polish, intuitive defaults. These are the ones users will hit in the first 5 minutes.

Lyrics and acapella are the ones that sound exciting to build. That's exactly why you should be suspicious of prioritizing them.

### 5. Recommendation

**Do Phase 1 now.** Then, instead of this wishlist, pick the **one area** that makes the existing experience feel broken or incomplete, and finish it completely before moving to the next.

My suggested order based on what I've seen in the codebase:

1. **Multi-folder music sources + config file** — your current `dirs::audio_dir()` single-folder assumption in [runtime.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-bin/src/runtime.rs#L134-L135) is the most limiting constraint. Without this, users with music in multiple locations simply can't use the app.

2. **Search/filter UX** — you have search working via commands (`:artist Adele`), but the discoverability is zero. Think about what the actual interaction should be before writing code.

3. **Playlist management polish** — you have create/delete/add/remove. What's missing? Test it as a user and find out, don't speculate.

4. **Help screen / onboarding** — make what already exists discoverable.

Everything else goes in a `ROADMAP.md`, not in your current sprint.

> [!IMPORTANT]
> Do not start the LLM, acapella, or music extraction features until items 1-4 are done and you've used the app daily for a week. You'll have much better instincts about what's actually missing after that.
