import ClickAway from "@/plugins/click-away";
import { createApp } from "vue";
import App from "./App.vue";
import "./styles.css";

const app = createApp(App);
app.use(ClickAway);
app.mount("#app");
