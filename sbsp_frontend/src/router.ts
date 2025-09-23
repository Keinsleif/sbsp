import { createWebHistory, createRouter } from 'vue-router';
import MainView from './MainView.vue';

const routes = [{ path: '/', component: MainView }];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
