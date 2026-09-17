import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// v2, deferred: the desktop app (apps/desktop, Tauri) is the primary target.
// This web build exists for later, and for most exchanges will need a thin
// proxy in front of it (browser CORS), unlike the desktop app which talks
// to exchanges directly from the Rust side.
export default defineConfig({
  plugins: [react()],
});
