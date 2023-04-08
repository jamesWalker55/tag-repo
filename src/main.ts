import { createApp } from "vue";
import "./styles.css";
import App from "./App.vue";
import VueVirtualScroller from "vue-virtual-scroller";

const app = createApp(App);
// app.use(VueVirtualScroller);
// app.use("virtual-list", VirtualList);
app.mount("#app");
