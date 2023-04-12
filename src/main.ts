import { createApp } from "vue";
import "./styles.css";
import App from "./App.vue";
import VueClickAwayPlugin from 'vue3-click-away';

const app = createApp(App);
app.use(VueClickAwayPlugin);
app.mount("#app");
