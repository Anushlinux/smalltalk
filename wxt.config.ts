import { defineConfig } from "wxt";

export default defineConfig({
  modules: ["@wxt-dev/module-react"],
  manifest: {
    name: "Smalltalk Resume Bookmark",
    short_name: "Smalltalk",
    description: "A smart research bookmark that restores your intent and exact reading position.",
    version: "0.1.0",
    permissions: ["activeTab", "storage", "tabs", "webNavigation"],
    host_permissions: ["http://localhost:8787/*"],
    action: {
      default_title: "Smalltalk Resume Bookmark",
      default_popup: "popup.html"
    }
  }
});
