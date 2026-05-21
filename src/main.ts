import { createApp } from "vue";
import { createPinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import App from "./App.vue";
import { router } from "./router";

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.mount("#app");

document.getElementById("initial-loader")?.remove();
invoke("close_splashscreen").catch((error) => {
  console.error("Impossible de fermer le splashscreen natif:", error);
});
