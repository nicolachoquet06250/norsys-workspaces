# Dev Workspace Manager

## 1) Cadrage fonctionnel (spec courte)

### Objectif produit (MVP)
- Permettre à un développeur de sélectionner un `Workspace` et de lancer automatiquement son environnement local (services + ressources à ouvrir).
- Réduire les actions manuelles de démarrage (terminaux, URLs, docs, outils).
- Permettre au développeur de gérer ses workspaces via un CRUD complet (créer, consulter, modifier, supprimer).

### Entités principales

#### Workspace
- `name` : nom lisible du workspace.
- `root` : chemin racine du projet.
- `services` : liste de services à démarrer.
- `open` : ressources à ouvrir au lancement (URLs, docs, onglets/outils).
- `envFiles` : fichiers d’environnement à charger.
- `variables env` : variables globales applicables au workspace.

#### Service
- `name` : identifiant du service.
- `command` : commande de lancement (obligatoire).
- `cwd` : dossier d’exécution.
- `env` : variables d’environnement spécifiques au service.
- `depends_on` : dépendances de démarrage.
- `mode` : `foreground` ou `background`.

#### Snapshot
- État restaurable de l’environnement (services actifs, contexte d’ouverture, état UI).
- Utilisé pour reprendre rapidement un contexte de travail.

### User story MVP
- En tant que développeur, je sélectionne un workspace ; l’application démarre les services dans le bon ordre puis ouvre automatiquement les URLs/docs/onglets requis.
- En tant que développeur, je peux créer un workspace, consulter la liste, modifier sa configuration et le supprimer sans éditer manuellement des fichiers.

### Gestion des Workspaces (CRUD)

#### Create
- Créer un nouveau `Workspace` depuis l’interface avec les champs minimum requis : `name`, `root` et au moins un `service`.

#### Read
- Consulter la liste des workspaces disponibles.
- Ouvrir le détail d’un workspace pour visualiser sa configuration (`services`, `open`, `envFiles`, variables d’environnement).

#### Update
- Modifier un workspace existant (nom, chemin racine, services, ressources à ouvrir, variables d’environnement).
- Les modifications sont validées avant sauvegarde et immédiatement reflétées dans la liste.

#### Delete
- Supprimer un workspace existant.
- Une confirmation utilisateur est demandée avant suppression définitive.

### Gestion des Services

#### Ajout et configuration
- Ajouter un service à un workspace avec les champs minimum requis : `name`, `command`.
- Configurer les champs optionnels : `cwd`, `env`, `depends_on`, `mode` (`foreground`/`background`).

#### Consultation
- Visualiser la liste des services d’un workspace et leur configuration.
- Afficher l’état runtime par service (`starting`, `running`, `failed`, `blocked`, `stopped`).

#### Mise à jour
- Modifier la configuration d’un service existant (commande, dossier, variables, dépendances, mode).
- Valider les champs requis avant sauvegarde (`name`, `command`) et empêcher les dépendances invalides.

#### Suppression
- Supprimer un service d’un workspace.
- Demander une confirmation avant suppression si le service est référencé dans `depends_on`.

#### Exécution et orchestration
- Permettre `Start`, `Stop`, `Restart` par service et `Start all`, `Stop all` par workspace.
- Respecter l’ordre de démarrage selon `depends_on`.
- En cas d’échec d’un service, marquer ses dépendants en `blocked`.

### Gestion des Snapshots

#### Création
- Permettre `save_snapshot(name)` pour sauvegarder l’état courant d’un workspace.
- Le snapshot inclut : services actifs, ressources ouvertes (`open`), et état UI minimal.

#### Consultation
- Lister les snapshots disponibles par workspace avec métadonnées minimales (`name`, date de création).
- Permettre la consultation du détail d’un snapshot avant restauration.

#### Restauration
- Permettre `restore_snapshot(name)` pour restaurer le contexte de travail.
- Si certains services ne sont plus disponibles, effectuer une restauration partielle avec message explicite.

#### Suppression
- Supprimer un snapshot existant avec confirmation utilisateur.
- La suppression d’un snapshot n’impacte pas la configuration du workspace.

### Comportements attendus en cas d’erreur

#### Service qui échoue au démarrage
- Le service passe à l’état `failed` avec message d’erreur explicite.
- Les autres services non dépendants peuvent continuer.
- Les services dépendants du service en échec ne démarrent pas et sont marqués `blocked`.

#### Timeout de démarrage
- Si un service ne devient pas opérationnel dans le délai imparti, il passe en `failed (timeout)`.
- L’échec est visible dans l’UI et journalisé.

#### Binaire/commande manquante
- Détection immédiate avant exécution réelle.
- Message actionnable : binaire introuvable + piste de correction.
- Aucune tentative répétée silencieuse.

### Critères d’acceptation (MVP)
- Sélectionner un workspace déclenche un lancement orchestré et observable.
- Chaque service affiche un état (`starting`, `running`, `failed`, `blocked`).
- Les ressources déclarées dans `open` s’ouvrent après le lancement.
- Les erreurs critiques sont explicites, traçables et n’empêchent pas la compréhension de l’état global.
- Le développeur peut effectuer les opérations CRUD sur les workspaces depuis l’application.
- La création et la mise à jour d’un workspace refusent les champs requis invalides ou vides (`name`, `root`, `command` des services).
- La suppression d’un workspace n’est exécutée qu’après confirmation explicite.
- Le développeur peut ajouter, modifier, supprimer et piloter les services d’un workspace avec validation des dépendances.
- Le développeur peut sauvegarder, lister, restaurer (même partiellement) et supprimer des snapshots depuis l’application.
