<script setup lang="ts">
import { DocFile } from '../../types';

defineProps<{
  nodes: DocFile[];
  expandedDirs: Set<string>;
  activePath: string;
}>();

const emit = defineEmits<{
  (e: 'nodeClick', node: DocFile): void;
}>();

function getDisplayName(name: string) {
  return name.replace('.md', '');
}
</script>

<template>
  <ul class="tree-list">
    <li v-for="node in nodes" :key="node.path" class="tree-node">
      <div 
        class="tree-node-content"
        :class="{ 
          'is-active': activePath === node.path || (node.path === 'README.md' && (activePath === '' || !activePath)),
          'is-dir': node.is_dir
        }"
        @click="emit('nodeClick', node)"
      >
        <span class="tree-toggler" v-if="node.is_dir">
          <svg 
            viewBox="0 0 24 24" 
            class="toggler-icon"
            :class="{ 'is-expanded': expandedDirs.has(node.path) }"
          >
            <path fill="currentColor" d="M8.59,16.58L13.17,12L8.59,7.41L10,6L16,12L10,18L8.59,16.58Z" />
          </svg>
        </span>
        <span class="tree-toggler-spacer" v-else></span>
        
        <span class="tree-icon">
          <template v-if="node.is_dir">
            <svg v-if="expandedDirs.has(node.path)" viewBox="0 0 24 24" class="icon-svg">
              <path fill="#e3b341" d="M19,20H4C2.89,20 2,19.1 2,18V6C2,4.89 2.89,4 4,4H10L12,6H19A2,2 0 0,1 21,8V18A2,2 0 0,1 19,20Z" />
            </svg>
            <svg v-else viewBox="0 0 24 24" class="icon-svg">
              <path fill="#e3b341" d="M10,4H4C2.89,4 2,4.89 2,6V18A2,2 0 0,0 4,20H19A2,2 0 0,0 21,18V8C21,6.89 20.1,6 19,6H12L10,4Z" />
            </svg>
          </template>
          <svg v-else viewBox="0 0 24 24" class="icon-svg">
            <path fill="#8b949e" d="M14,2H6A2,2 0 0,0 4,4V20A2,2 0 0,0 6,22H18A2,2 0 0,0 20,20V8L14,2M18,20H6V4H13V9H18V20Z" />
          </svg>
        </span>
        
        <span class="tree-node-label">{{ getDisplayName(node.name) }}</span>
      </div>
      
      <transition name="tree-slide">
        <div v-if="node.is_dir && expandedDirs.has(node.path)" class="tree-node-children">
          <DocTree 
            :nodes="node.children || []" 
            :expanded-dirs="expandedDirs"
            :active-path="activePath"
            @node-click="(n) => emit('nodeClick', n)"
          />
        </div>
      </transition>
    </li>
  </ul>
</template>

<style scoped>
.tree-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.tree-node {
  outline: none;
}

.tree-node-content {
  display: flex;
  align-items: center;
  padding: 0.25rem 0.5rem;
  border-radius: 6px;
  cursor: pointer;
  transition: background-color 0.2s, box-shadow 0.2s;
  user-select: none;
  margin-bottom: 2px;
}

.tree-node-content:hover {
  background-color: rgba(255, 255, 255, 0.05);
}

.tree-node-content.is-active {
  background-color: rgba(31, 111, 235, 0.15);
  color: #58a6ff;
}

.tree-toggler {
  width: 1.5rem;
  height: 1.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-right: 0.25rem;
  border-radius: 4px;
  transition: background-color 0.2s;
  color: #8b949e;
}

.tree-toggler:hover {
  background-color: rgba(255, 255, 255, 0.1);
}

.toggler-icon {
  width: 1.25rem;
  height: 1.25rem;
  transition: transform 0.2s;
}

.toggler-icon.is-expanded {
  transform: rotate(90deg);
}

.tree-toggler-spacer {
  width: 1.75rem;
}

.tree-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  margin-right: 0.5rem;
}

.icon-svg {
  width: 1.1rem;
  height: 1.1rem;
}

.tree-node-label {
  font-size: 0.9rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tree-node-children {
  padding-left: 1.15rem;
}

/* Animation */
.tree-slide-enter-active,
.tree-slide-leave-active {
  transition: all 0.3s ease-out;
  max-height: 1000px;
  overflow: hidden;
}

.tree-slide-enter-from,
.tree-slide-leave-to {
  max-height: 0;
  opacity: 0;
}
</style>

<script lang="ts">
export default {
  name: 'DocTree'
}
</script>
