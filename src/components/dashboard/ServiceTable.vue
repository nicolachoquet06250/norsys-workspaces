<script setup lang="ts">
import type { WorkspaceConfig, ServiceConfig, ServiceRuntimeStatus } from "../../types";
import {defineAsyncComponent} from "vue";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

interface ServiceItem {
  config: ServiceConfig;
  workspace: WorkspaceConfig;
  status: ServiceRuntimeStatus;
  port?: string;
  accessUrl?: string;
  lastActivity?: string;
}

withDefaults(defineProps<{
  services: ServiceItem[];
  showWorkspace?: boolean;
  showViewAllLink?: boolean;
  showAccessUrlColumn?: boolean;
}>(), {
  showWorkspace: true,
  showViewAllLink: true,
  showAccessUrlColumn: false,
});

function getTechIcon(name: string, kind?: string) {
  const n = name.toLowerCase();
  const k = kind?.toLowerCase();

  if (n.includes('node')) return defineAsyncComponent(() => import('../../icons/nodejs.vue'));
  if (n.match(/_js|js_|^js$/) || n.includes('javascript')) return defineAsyncComponent(() => import('../../icons/js.vue'));
  if (n.match(/_ts|ts_|^ts$/) || n.includes('typescript')) return defineAsyncComponent(() => import('../../icons/ts.vue'));
  if (n.includes('python')) return defineAsyncComponent(() => import('../../icons/python.vue'));
  if (n.includes('php')) return defineAsyncComponent(() => import('../../icons/php.vue'));
  if (n.includes('java')) return defineAsyncComponent(() => import('../../icons/java.vue'));
  if (n.includes('golang')) return defineAsyncComponent(() => import('../../icons/golang.vue'));
  if (n.includes('csharp') || n.match(/_c#|c#_|^c#$/) || n.includes('dotnet')) return defineAsyncComponent(() => import('../../icons/csharp.vue'));

  if (n.includes('react')) return defineAsyncComponent(() => import('../../icons/react.vue'));
  if (n.includes('vue')) return defineAsyncComponent(() => import('../../icons/vue.vue'));
  if (n.includes('angular')) return defineAsyncComponent(() => import('../../icons/angular.vue'));
  if (n.includes('svelte')) return defineAsyncComponent(() => import('../../icons/svelte.vue'));

  if (n.includes('postgres')) return defineAsyncComponent(() => import('../../icons/postgres.vue'));
  if (n.includes('mysql')) return defineAsyncComponent(() => import('../../icons/mysql.vue'));
  if (n.includes('mariadb')) return defineAsyncComponent(() => import('../../icons/mariadb.vue'));
  if (n.includes('redis')) return defineAsyncComponent(() => import('../../icons/redis.vue'));

  if (k === 'database' || n.includes('sql')) return defineAsyncComponent(() => import('../../icons/database.vue'));

  return defineAsyncComponent(() => import('../../icons/web.vue'))
}

function getStatusClass(status: ServiceRuntimeStatus) {
  return status;
}

function getStatusLabel(status: ServiceRuntimeStatus) {
  switch (status) {
    case 'running': return 'En cours';
    case 'starting': return 'Démarrage';
    case 'stopping': return 'Arrêt';
    case 'stopped': return 'Arrêté';
    case 'failed': return 'Erreur';
    default: return 'Idle';
  }
}

function openTerminal(item: ServiceItem) {
  const sanitizeWindowLabelPart = (value: string) => value.replace(/[^a-zA-Z0-9\-/:_]/g, "-");
  const safeWorkspaceId = sanitizeWindowLabelPart(item.workspace.id);
  const safeServiceName = sanitizeWindowLabelPart(item.config.name);
  const label = `terminal-${safeWorkspaceId}-${safeServiceName}-${Date.now()}`;
  const workspaceId = encodeURIComponent(item.workspace.id);
  const serviceName = encodeURIComponent(item.config.name);
  const workspaceName = item.workspace.name;
  const title = item.config.display_name || item.config.name;

  const terminalWindow = new WebviewWindow(label, {
    title: `Terminal · ${workspaceName} · ${title}`,
    url: `/#/terminal?workspaceId=${workspaceId}&serviceName=${serviceName}`,
    width: 1000,
    height: 700,
    center: true,
    focus: true,
    alwaysOnTop: true,
  });

  terminalWindow.once("tauri://created", async () => {
    try {
      await terminalWindow.setFocus();
      window.setTimeout(async () => {
        try {
          await terminalWindow.setAlwaysOnTop(false);
        } catch (error) {
          console.error("Impossible de rétablir le z-order de la fenêtre terminal:", error);
        }
      }, 150);
    } catch (error) {
      console.error("Impossible de forcer le focus de la fenêtre terminal:", error);
    }
  });

  terminalWindow.once("tauri://error", (event) => {
    console.error("Impossible d'ouvrir la fenêtre terminal:", event);
  });
}
</script>

<template>
  <div class="service-table-container">
    <table v-if="services.length > 0" class="service-table">
      <thead>
        <tr>
          <th>Service</th>
          <th v-if="showWorkspace" class="hide-mobile">Workspace</th>
          <th class="hide-tablet">Image</th>
          <th>Statut</th>
          <th class="hide-mobile">Port(s)</th>
          <th v-if="showAccessUrlColumn" class="hide-mobile">URL d'accès</th>
          <th class="hide-tablet">Dernière activité</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="item in services" :key="item.workspace.id + item.config.name">
          <td>
            <div class="service-name">
              <span class="tech-icon">
                <Suspense>
                  <component :is="getTechIcon(item.config.name, item.config.kind)" :width="24" :height="24" />

                  <template #fallback>
                    <div class="loader"/>
                  </template>
                </Suspense>
              </span>
              <span>{{ item.config.display_name || item.config.name }}</span>
            </div>
          </td>
          <td v-if="showWorkspace" class="hide-mobile">
            <router-link
              class="workspace-link"
              :to="{ name: 'workspace-detail', params: { id: item.workspace.id } }"
            >
              {{ item.workspace.name }}
            </router-link>
          </td>
          <td class="hide-tablet">
            <div class="image-tag">
              <a v-if="item.config.image" class="access-link" :href="`https://hub.docker.com/layers/library/${item.config.image.split(':')[0]}/${item.config.image.split(':')[1]}`" target="_blank" rel="noopener noreferrer">
                {{ item.config.image }}
              </a>
              <template v-else>----</template>
            </div>
          </td>
          <td>
            <div class="status-cell">
              <span class="status-dot" :class="getStatusClass(item.status)"></span>
              <span class="status-label" :class="getStatusClass(item.status)">{{ getStatusLabel(item.status) }}</span>
            </div>
          </td>
          <td class="hide-mobile"><span class="port">{{ item.port ?? '----' }}</span></td>
          <td v-if="showAccessUrlColumn" class="hide-mobile">
            <a
              v-if="item.accessUrl"
              class="access-link"
              :href="item.accessUrl"
              target="_blank"
              rel="noopener noreferrer"
            >
              {{ item.accessUrl }}
            </a>
            <span v-else class="port">----</span>
          </td>
          <td class="activity hide-tablet">{{ item.lastActivity || 'Il y a 2 min' }}</td>
          <td>
            <button
              v-if="item.status === 'running'"
              class="action-btn"
              type="button"
              title="Ouvrir le terminal"
              aria-label="Ouvrir le terminal"
              @click="openTerminal(item)"
            >
              🖥️
            </button>
          </td>
        </tr>
      </tbody>
    </table>
    <div v-else class="empty-state">
      <p>Aucun service à afficher.</p>
    </div>
    <div v-if="showViewAllLink && services.length > 0" class="footer">
      <router-link to="/services" class="view-all">Voir tous les services</router-link>
    </div>
  </div>
</template>

<style scoped>
.service-table-container {
  background-color: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  overflow-x: auto;
}

.service-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.85rem;
  text-align: left;
  min-width: 800px;
}

.service-table th {
  color: #8b949e;
  font-weight: 500;
  padding: 1rem;
  border-bottom: 1px solid #30363d;
}

.service-table td {
  padding: 1rem;
  color: #f0f6fc;
  border-bottom: 1px solid #21262d;
}

.service-name {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-weight: 500;
}

.image-tag {
  background-color: #0d1117;
  padding: 0.25rem 0.5rem;
  border-radius: 6px;
  font-size: 0.75rem;
  border: 1px solid #30363d;
  display: inline-block;
  font-family: monospace;
  color: #8b949e;
}

.status-cell {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: #8b949e;
}

.status-label { color: #8b949e; }

.port {
  color: #58a6ff;
  font-family: monospace;
}

.activity {
  color: #8b949e;
}

.workspace-link {
  color: #79c0ff;
  text-decoration: none;
}

.workspace-link:visited {
  color: #79c0ff;
}

.workspace-link:hover {
  text-decoration: underline;
}

.access-link {
  color: #79c0ff;
  text-decoration: none;
}

.access-link:visited {
  color: #79c0ff;
}

.access-link:hover {
  text-decoration: underline;
}

.action-btn {
  background: none;
  border: none;
  color: #8b949e;
  cursor: pointer;
}

.footer {
  padding: 0.75rem 1rem;
}

.view-all {
  color: #58a6ff;
  text-decoration: none;
  font-size: 0.8rem;
}

.view-all:hover {
  text-decoration: underline;
}

.empty-state {
  padding: 2rem;
  text-align: center;
}

.empty-state p {
  color: #8b949e;
  font-size: 0.9rem;
  margin: 0;
}

@media (max-width: 900px) {
  .hide-tablet {
    display: none;
  }
}

@media (max-width: 600px) {
  .hide-mobile {
    display: none;
  }
  
  .service-table {
    min-width: unset;
  }

  .service-table th, .service-table td {
    padding: 0.75rem 0.5rem;
  }
}

.loader {
  width: 50px;
  aspect-ratio: 1;
  border-radius: 50%;
  border: 8px solid #ffffff;
  animation:
      l20-1 0.8s infinite linear alternate,
      l20-2 1.6s infinite linear;
}
@keyframes l20-1{
  0%    {clip-path: polygon(50% 50%,0       0,  50%   0%,  50%    0%, 50%    0%, 50%    0%, 50%    0% )}
  12.5% {clip-path: polygon(50% 50%,0       0,  50%   0%,  100%   0%, 100%   0%, 100%   0%, 100%   0% )}
  25%   {clip-path: polygon(50% 50%,0       0,  50%   0%,  100%   0%, 100% 100%, 100% 100%, 100% 100% )}
  50%   {clip-path: polygon(50% 50%,0       0,  50%   0%,  100%   0%, 100% 100%, 50%  100%, 0%   100% )}
  62.5% {clip-path: polygon(50% 50%,100%    0, 100%   0%,  100%   0%, 100% 100%, 50%  100%, 0%   100% )}
  75%   {clip-path: polygon(50% 50%,100% 100%, 100% 100%,  100% 100%, 100% 100%, 50%  100%, 0%   100% )}
  100%  {clip-path: polygon(50% 50%,50%  100%,  50% 100%,   50% 100%,  50% 100%, 50%  100%, 0%   100% )}
}
@keyframes l20-2{
  0%    {transform:scaleY(1)  rotate(0deg)}
  49.99%{transform:scaleY(1)  rotate(135deg)}
  50%   {transform:scaleY(-1) rotate(0deg)}
  100%  {transform:scaleY(-1) rotate(-135deg)}
}
</style>
