import {createApp} from 'vue';
import './style.css';
import App from './App.vue';
import VueClickAway from "vue3-click-away";

const app = createApp(App);
app.use(VueClickAway);
app.mount('#app');
