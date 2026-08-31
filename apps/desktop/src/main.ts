import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { i18n, initializeLocale } from "./i18n";

const app = createApp(App);

initializeLocale();
app.use(i18n);
app.use(createPinia());
app.mount("#app");
