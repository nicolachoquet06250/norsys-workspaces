<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useDocumentationStore } from '../../stores/documentation';
import { marked } from 'marked';
import DocTree from './DocTree.vue';
import { DocFile } from '../../types';

const props = defineProps<{
  workspaceId: string;
}>();

const route = useRoute();
const router = useRouter();
const docStore = useDocumentationStore();

const content = ref('');
const isLoading = ref(false);
const error = ref<string | null>(null);
const expandedDirs = ref<Set<string>>(new Set());

function toggleDir(path: string) {
  if (expandedDirs.value.has(path)) {
    expandedDirs.value.delete(path);
  } else {
    expandedDirs.value.add(path);
  }
}

function expandToPath(path: string) {
  if (!path) return;
  const parts = path.split('/');
  let current = '';
  for (let i = 0; i < parts.length - 1; i++) {
    current = current ? `${current}/${parts[i]}` : parts[i];
    expandedDirs.value.add(current);
  }
}

async function loadContent() {
  const docPath = route.params.path as string || 'README.md';
  isLoading.value = true;
  error.value = null;
  
  // S'assurer que les dossiers parents sont dépliés
  expandToPath(docPath);
  
  try {
    const rawContent = await docStore.readDocFile(docPath, props.workspaceId);
    
    // Configurer marked pour gérer les images relatives
    const currentDir = docPath.includes('/') 
      ? docPath.substring(0, docPath.lastIndexOf('/')) 
      : '';

    const renderer = new marked.Renderer();

    renderer.image = (token) => {
      let { href, title, text } = token;
      
      // Check for dimensions in href like url =100x200 or url =100x
      const dimMatch = href.match(/=([0-9]+)?x([0-9]+)?$/);
      let width = '';
      let height = '';
      if (dimMatch) {
        href = href.substring(0, dimMatch.index).trim();
        width = dimMatch[1] || '';
        height = dimMatch[2] || '';
      }
      
      const isRelative = !href.startsWith('http') && !href.startsWith('//') && !href.startsWith('data:');
      
      // Si c'est un lien relatif
      let normalizedPath = href;
      if (isRelative) {
        // Résoudre le chemin de l'image par rapport au fichier MD
        let imagePath = href;
        if (href.startsWith('./')) {
          imagePath = href.substring(2);
        }
        
        const fullRelativePath = currentDir ? `${currentDir}/${imagePath}` : imagePath;
        
        // Normaliser le chemin
        const parts = fullRelativePath.split('/');
        const resolvedParts: string[] = [];
        for (const part of parts) {
          if (part === '.' || part === '') continue;
          if (part === '..') resolvedParts.pop();
          else resolvedParts.push(part);
        }
        normalizedPath = resolvedParts.join('/');
      }
      
      const style = [
        width ? `width: ${width}${isNaN(Number(width)) ? '' : 'px'}` : '',
        height ? `height: ${height}${isNaN(Number(height)) ? '' : 'px'}` : ''
      ].filter(Boolean).join('; ');

      // On va utiliser un conteneur pour afficher un loader pendant le chargement
      return `
        <span class="img-container loading" 
              data-href="${href}" 
              data-is-relative="${isRelative}" 
              data-raw-path="${isRelative ? normalizedPath : ''}" 
              data-alt="${text}" 
              data-title="${title || ''}"
              style="${style}">
          <span class="img-loader"></span>
          <img src="" alt="${text}" title="${title || ''}" class="lazy-doc-img" style="display: none; ${style}">
        </span>
      `.trim();
    };

    content.value = await marked.parse(rawContent, { renderer });
    
    // Charger les images après le rendu
    setTimeout(loadImages, 0);

    // Faire remonter le scroll après le changement de contenu
    document.querySelector('.docs-content')?.scrollTo(0, 0);
  } catch (err) {
    console.error(err);
    error.value = "Impossible de charger le fichier de documentation.";
    content.value = '';
  } finally {
    isLoading.value = false;
  }
}

async function loadImages() {
  const containers = document.querySelectorAll('.img-container.loading') as NodeListOf<HTMLElement>;
  for (const container of containers) {
    const isRelative = container.getAttribute('data-is-relative') === 'true';
    const href = container.getAttribute('data-href');
    const rawPath = container.getAttribute('data-raw-path');
    const img = container.querySelector('.lazy-doc-img') as HTMLImageElement;
    
    if (img) {
      const handleLoad = () => {
        container.classList.remove('loading');
        img.style.display = 'inline-block';
        const loader = container.querySelector('.img-loader');
        if (loader) loader.remove();
      };

      const handleError = () => {
        container.classList.remove('loading');
        container.classList.add('error');
        const loader = container.querySelector('.img-loader');
        if (loader) loader.remove();
        
        // Créer un placeholder avec les dimensions si possible
        const alt = container.getAttribute('data-alt') || 'Image';
        const path = rawPath || href || '';
        container.innerHTML = `<span class="img-placeholder"><span>🖼️ ${alt}</span><small>(${path})</small></span>`;
      };

      if (isRelative && rawPath) {
        try {
          const bytes = await docStore.getDocImage(rawPath, props.workspaceId);
          const ext = rawPath.split('.').pop()?.toLowerCase() || 'png';
          const mimeType = ext === 'svg' ? 'image/svg+xml' : `image/${ext}`;
          const blob = new Blob([bytes], { type: mimeType });
          const url = URL.createObjectURL(blob);
          
          img.onload = handleLoad;
          img.onerror = handleError;
          img.src = url;
        } catch (err) {
          console.error(`Failed to load image: ${rawPath}`, err);
          handleError();
        }
      } else if (href) {
        img.onload = handleLoad;
        img.onerror = handleError;
        img.src = href;
      }
    }
  }
}

onMounted(async () => {
  await docStore.fetchDocs(props.workspaceId);
  await loadContent();
  
  // Intercepter les clics sur les liens pour la navigation interne
  document.addEventListener('click', handleLinkClick);
});

onUnmounted(() => {
  document.removeEventListener('click', handleLinkClick);
});

watch(() => route.params.path, () => {
  loadContent();
});

watch(() => props.workspaceId, async (newId) => {
  await docStore.fetchDocs(newId);
  await loadContent();
});

function handleLinkClick(e: MouseEvent) {
  const target = e.target as HTMLElement;
  const link = target.closest('a');
  
  if (link && link.getAttribute('href')?.endsWith('.md')) {
    const href = link.getAttribute('href')!;
    // Si c'est un lien relatif vers un autre fichier markdown
    if (!href.startsWith('http') && !href.startsWith('//')) {
      e.preventDefault();
      
      // Calculer le nouveau chemin
      // Chemin absolu par rapport à la racine de la doc (commence par /)
      let newPath = href.startsWith('/') ? href.substring(1) : href;
      const currentPath = route.params.path as string || 'README.md';
      
      // Si le lien est relatif (ne commence pas par /)
      if (!href.startsWith('/')) {
        const currentDir = currentPath.includes('/')
          ? currentPath.substring(0, currentPath.lastIndexOf('/'))
          : '';

        const fullRelativePath = currentDir ? `${currentDir}/${href}` : href;
        const parts = fullRelativePath.split('/');
        const resolvedParts: string[] = [];

        for (const part of parts) {
          if (part === '.' || part === '') {
            continue;
          } else if (part === '..') {
            resolvedParts.pop();
            continue;
          }

          resolvedParts.push(part);
        }

        newPath = resolvedParts.join('/');
      }
      
      // Normaliser le chemin (enlever les doubles slashs éventuels)
      newPath = newPath.replace(/\/+/g, '/');
      
      router.push({ name: 'workspace-docs', params: { id: props.workspaceId, path: newPath } });
    }
  }
}

function navigateTo(path: string) {
  router.push({ name: 'workspace-docs', params: { id: props.workspaceId, path } });
}

function handleDocClick(doc: DocFile) {
  if (doc.is_dir) {
    toggleDir(doc.path);
  } else {
    navigateTo(doc.path);
  }
}

async function refreshDocs() {
  await docStore.fetchDocs(props.workspaceId);
  await loadContent();
}
</script>

<template>
  <div class="docs-view">
    <div class="docs-sidebar">
      <div class="sidebar-header">
        <h3>Documentation</h3>
        <button 
          @click="refreshDocs" 
          class="refresh-btn" 
          title="Actualiser la documentation"
          :disabled="docStore.isLoading || isLoading"
        >
          <span :class="{ 'spinning': docStore.isLoading || isLoading }">🔄</span>
        </button>
      </div>
      <div v-if="docStore.isLoading" class="loading-side">Chargement...</div>
      <div v-else-if="docStore.hasDocs" class="tree-container">
        <DocTree 
          :nodes="docStore.docs" 
          :expanded-dirs="expandedDirs"
          :active-path="(route.params.path as string) || ''"
          @node-click="handleDocClick"
        />
      </div>
      <div v-else class="no-docs">
        Aucun fichier markdown trouvé.
      </div>
    </div>
    
    <div class="docs-content">
      <div v-if="isLoading" class="loading-content">
        <div class="spinner"></div>
        <p>Chargement de la documentation...</p>
      </div>
      
      <div v-else-if="error" class="error-content">
        <div class="error-icon">⚠️</div>
        <p>{{ error }}</p>
        <button @click="loadContent" class="retry-btn">Réessayer</button>
      </div>
      
      <div v-else class="markdown-body" v-html="content"></div>
    </div>
  </div>
</template>

<style scoped>
.docs-view {
  display: flex;
  height: 550px;
  background-color: #0d1117;
  color: #c9d1d9;
  border-radius: 8px;
  border: 1px solid #30363d;
  overflow: hidden;
}

.docs-sidebar {
  width: 250px;
  border-right: 1px solid #30363d;
  padding: 1.5rem 1rem;
  overflow-y: auto;
  flex-shrink: 0;
}

.sidebar-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
  padding-left: 0.5rem;
}

.docs-sidebar h3 {
  font-size: 0.9rem;
  text-transform: uppercase;
  color: #8b949e;
  margin: 0;
}

.refresh-btn {
  background: none;
  border: none;
  color: #8b949e;
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.refresh-btn:hover:not(:disabled) {
  background-color: #21262d;
  color: #58a6ff;
}

.refresh-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.spinning {
  display: inline-block;
  animation: spin 1s linear infinite;
}

.tree-container {
  display: flex;
  flex-direction: column;
}

.no-docs {
  padding: 1rem;
  color: #8b949e;
  font-size: 0.9rem;
  font-style: italic;
}

.docs-content {
  flex-grow: 1;
  padding: 0 3rem;
  overflow-y: auto;
  scroll-behavior: smooth;
}

.loading-content, .error-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #8b949e;
}

.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid #30363d;
  border-top-color: #58a6ff;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: 1rem;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.error-icon {
  font-size: 3rem;
  margin-bottom: 1rem;
}

.retry-btn {
  margin-top: 1rem;
  padding: 0.5rem 1rem;
  background-color: #21262d;
  border: 1px solid #30363d;
  color: #c9d1d9;
  border-radius: 6px;
  cursor: pointer;
}

.retry-btn:hover {
  background-color: #30363d;
}

/* GitHub-like Markdown styles */
:deep(.markdown-body) {
  line-height: 1.6;
}

:deep(.markdown-body h1), 
:deep(.markdown-body h2) {
  border-bottom: 1px solid #30363d;
  padding-bottom: 0.3em;
  margin-top: 24px;
  margin-bottom: 16px;
  font-weight: 600;
}

:deep(.markdown-body h1) { font-size: 2em; }
:deep(.markdown-body h2) { font-size: 1.5em; }

:deep(.markdown-body a) {
  color: #58a6ff;
  text-decoration: none;
}

:deep(.markdown-body a:hover) {
  text-decoration: underline;
}

:deep(.markdown-body code) {
  padding: 0.2em 0.4em;
  margin: 0;
  font-size: 85%;
  background-color: rgba(110, 118, 129, 0.4);
  border-radius: 6px;
  font-family: ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace;
}

:deep(.markdown-body pre) {
  padding: 16px;
  overflow: auto;
  font-size: 85%;
  line-height: 1.45;
  background-color: #161b22;
  border-radius: 6px;
  margin-bottom: 16px;
}

:deep(.markdown-body pre code) {
  background-color: transparent;
  padding: 0;
}

:deep(.markdown-body img) {
  max-width: 100%;
}

:deep(.img-container) {
  display: inline-block;
  position: relative;
  min-width: 50px;
  min-height: 50px;
  background-color: #161b22;
  border-radius: 6px;
  overflow: hidden;
  vertical-align: middle;
  margin-bottom: 16px;
}

:deep(.img-container.loading) {
  border: 1px solid #30363d;
}

:deep(.img-container.error) {
  border: 1px dashed #f85149;
  padding: 1rem;
  text-align: center;
}

:deep(.img-loader) {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 24px;
  height: 24px;
  border: 2px solid #30363d;
  border-top-color: #58a6ff;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

:deep(.img-placeholder) {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #8b949e;
  font-size: 0.9rem;
  gap: 0.5rem;
  height: 100%;
}

:deep(.img-placeholder small) {
  font-size: 0.75rem;
  opacity: 0.7;
  word-break: break-all;
}

:deep(.lazy-doc-img) {
  max-width: 100%;
  height: auto;
  display: block;
}

:deep(.markdown-body ul), 
:deep(.markdown-body ol) {
  padding-left: 2em;
  margin-bottom: 16px;
}

:deep(.markdown-body p) {
  margin-bottom: 16px;
}
</style>
