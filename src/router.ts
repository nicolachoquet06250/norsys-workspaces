import { createRouter, createWebHashHistory } from "vue-router";
import HomeView from "./views/HomeView.vue";
import CreateWorkspaceView from "./views/CreateWorkspaceView.vue";
import WorkspaceDetailView from "./views/WorkspaceDetailView.vue";

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "home",
      component: HomeView,
    },
    {
      path: "/workspace/:id",
      name: "workspace-detail",
      component: WorkspaceDetailView,
    },
    {
      path: "/workspace/new",
      name: "workspace-create",
      component: CreateWorkspaceView,
    },
  ],
});
