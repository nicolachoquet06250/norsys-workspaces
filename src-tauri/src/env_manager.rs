use std::collections::HashMap;
use std::fs;
use std::path::{PathBuf};
use regex::Regex;

use crate::workspace_loader::WorkspaceConfig;

pub fn get_env_files(root: &str) -> Vec<String> {
    let mut files = Vec::new();
    let re = Regex::new(r"^\.env(\.[a-zA-Z0-9_\-]+)?(\.local)?$").unwrap();

    // 1. Scanner la racine
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if re.is_match(&file_name) {
                        files.push(file_name);
                    }
                }
            } else if path.is_dir() {
                // 2. Scanner les sous-répertoires de niveau 1
                if let Ok(sub_entries) = fs::read_dir(&path) {
                    let dir_name = entry.file_name().into_string().unwrap_or_default();
                    if dir_name == "node_modules" || dir_name == "target" || dir_name.starts_with('.') {
                        continue;
                    }

                    for sub_entry in sub_entries.flatten() {
                        if sub_entry.path().is_file() {
                            if let Ok(sub_file_name) = sub_entry.file_name().into_string() {
                                if re.is_match(&sub_file_name) {
                                    let mut relative_path = PathBuf::from(dir_name.clone());
                                    relative_path.push(sub_file_name);
                                    if let Some(path_str) = relative_path.to_str() {
                                        files.push(path_str.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Trier les fichiers pour respecter l'ordre de priorité
    // Les fichiers à la racine sont prioritaires sur les fichiers dans les sous-répertoires ?
    // En général, dans un monorepo, les .env des sous-projets sont spécifiques au sous-projet.
    // Si on les fusionne pour tout le workspace, on doit décider de l'ordre.
    // Conventionnellement : .env < .env.local < .env.<mode> < .env.<mode>.local
    // Et racine < sous-répertoire (pour que le sous-projet surcharge la racine si besoin, ou l'inverse ?)
    // Souvent la racine contient des trucs globaux et les sous-projets surchargent.
    // Mais ici on veut peut être que la racine soit le maitre.
    // Docker compose prend les fichiers dans l'ordre de la ligne de commande, le dernier gagne.
    
    files.sort_by(|a, b| {
        let is_root = |name: &str| !name.contains(std::path::MAIN_SEPARATOR) && !name.contains('/');
        
        let score = |name: &str| {
            let base_name = if let Some(idx) = name.rfind(|c| c == '/' || c == std::path::MAIN_SEPARATOR) {
                &name[idx + 1..]
            } else {
                name
            };

            let priority = if base_name == ".env" {
                0
            } else if base_name == ".env.local" {
                1
            } else if base_name.ends_with(".local") {
                3
            } else {
                2
            };
            
            // On veut que la racine soit traitée EN PREMIER (donc score plus bas) 
            // pour que les sous-répertoires surchargent ? 
            // Ou l'inverse ? 
            // Si on suit Docker Compose --env-file, le dernier fichier passé surcharge les précédents.
            // Donc si on veut que le sous-répertoire surcharge la racine : racine d'abord, puis sous-répertoire.
            let root_score = if is_root(name) { 0 } else { 1 };
            
            (root_score, priority)
        };
        
        score(a).cmp(&score(b)).then(a.cmp(b))
    });

    files
}

pub fn resolve_workspace_env(workspace: &WorkspaceConfig) -> HashMap<String, String> {
    let mut resolved = HashMap::new();

    // 1. Charger les variables d'environnement depuis les fichiers .env
    let env_files = get_env_files(&workspace.root);

    for file_name in env_files {
        let mut path = PathBuf::from(&workspace.root);
        path.push(file_name);

        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }

                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim().to_string();
                        let value = value.trim().to_string();
                        
                        // Supprimer les guillemets éventuels autour de la valeur
                        let value = if (value.starts_with('"') && value.ends_with('"')) || 
                                       (value.starts_with('\'') && value.ends_with('\'')) {
                            value[1..value.len() - 1].to_string()
                        } else {
                            value
                        };

                        resolved.insert(key, value);
                    }
                }
            }
        }
    }

    // 2. Surcharger avec les variables définies explicitement dans le workspace
    for (key, value) in &workspace.env {
        resolved.insert(key.clone(), value.clone());
    }

    // 3. Ajouter les variables système/workspace
    resolved.insert("WORKSPACE_NAME".to_string(), workspace.name.clone());
    resolved.insert("WORKSPACE_ROOT".to_string(), workspace.root.clone());

    resolved
}
