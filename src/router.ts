import { createRouter, createWebHashHistory } from "vue-router";
import HomeView from "./views/HomeView.vue";
import CreateWorkspaceView from "./views/CreateWorkspaceView.vue";
import WorkspaceDetailView from "./views/WorkspaceDetailView.vue";
import WorkspacesView from "./views/WorkspacesView.vue";
import ServicesView from "./views/ServicesView.vue";
import ImagesView from "./views/ImagesView.vue";
import VolumesView from "./views/VolumesView.vue";

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
    {
      path: "/workspaces",
      name: "workspaces",
      component: WorkspacesView,
    },
    {
      path: "/services",
      name: "services",
      component: ServicesView,
    },
    {
      path: "/images",
      name: "images",
      component: ImagesView,
    },
    {
      path: "/volumes",
      name: "volumes",
      component: VolumesView,
    },
  ],
});
