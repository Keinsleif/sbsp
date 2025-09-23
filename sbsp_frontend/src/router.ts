import { createWebHistory, createRouter } from 'vue-router';
import MainView from './MainView.vue';
import SettingsView from './SettingsView.vue';

const routes = [
  { path: '/', component: MainView },
  { path: '/settings', component: SettingsView },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
