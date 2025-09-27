import { createWebHistory, createRouter } from 'vue-router';
import MainView from './MainView.vue';
import ConnectView from './ConnectView.vue';
import FileListView from './FileListView.vue';
import ServerPanelView from './ServerPanelView.vue';

const routes = [
  { path: '/', component: MainView },
  { path: '/connect', component: ConnectView },
  { path: '/pick_file', component: FileListView },
  { path: '/server', component: ServerPanelView },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
