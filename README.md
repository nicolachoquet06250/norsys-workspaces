# Tauri + Vue + TypeScript

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

## Auto-update Configuration

Le système d'auto-update a été configuré avec le plugin `@tauri-apps/plugin-updater`.

### Étapes pour finaliser la configuration :

1. **Générer les clés de signature :**
   Exécutez la commande suivante à la racine du projet pour générer une paire de clés :
   ```bash
   npx tauri signer generate -w src-tauri/main.key
   ```

2. **Mettre à jour la clé publique :**
   Copiez la clé publique générée et remplacez la valeur `"DWN_YOUR_PUBLIC_KEY_HERE"` dans le fichier `src-tauri/tauri.conf.json`.

3. **Configurer les secrets GitHub :**
   Dans les paramètres de votre dépôt GitHub (Settings > Secrets and variables > Actions), ajoutez les secrets suivants :
   - `TAURI_SIGNING_PRIVATE_KEY` : Le contenu du fichier `src-tauri/main.key`.
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` : Le mot de passe utilisé lors de la génération de la clé (si applicable).
   - `GH_TOKEN` : Un Personal Access Token avec les permissions `contents:write`.

### Fonctionnement :
- L'application vérifie automatiquement les mises à jour au démarrage (configuré avec `dialog: true`).
- Les mises à jour sont récupérées depuis les releases GitHub.
- Le workflow GitHub Actions génère automatiquement le fichier `latest.json` requis par l'updater.
