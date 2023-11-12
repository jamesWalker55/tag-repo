import { createApp } from "vue";
import "./styles.css";
import App from "./App.vue";
import ClickAway from "@/plugins/click-away";

const app = createApp(App);
app.use(ClickAway);
app.mount("#app");
