use std::fs;
use std::path::{PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DocFile {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<DocFile>>,
}

fn get_workspace_root(workspace_id: Option<String>) -> Result<PathBuf, String> {
    if let Some(id) = workspace_id {
        // Chercher la racine du workspace via son ID
        let workspaces = crate::workspace_loader::list_workspaces()
            .map_err(|e| e.to_string())?;
        
        let workspace = workspaces.into_iter()
            .find(|w| w.id == id)
            .ok_or_else(|| format!("Workspace introuvable: {}", id))?;
        
        return Ok(PathBuf::from(workspace.root));
    }

    let mut root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    
    // Si on est dans src-tauri, on remonte d'un cran
    if root.ends_with("src-tauri") {
        root.pop();
    }
    
    Ok(root)
}

#[tauri::command]
pub fn list_docs(workspace_id: Option<String>) -> Result<Vec<DocFile>, String> {
    let root = get_workspace_root(workspace_id)?;
    let mut docs = Vec::new();

    // 1. Chercher les fichiers MD à la racine
    if let Ok(entries) = fs::read_dir(&root) {
        let mut root_mds = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    root_mds.push(DocFile {
                        name: name.to_string(),
                        path: name.to_string(),
                        is_dir: false,
                        children: None,
                    });
                }
            }
        }
        // Trier les fichiers MD de la racine par nom
        root_mds.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        docs.extend(root_mds);
    }

    // 2. Chercher récursivement dans /docs ou /doc
    for dir_name in &["docs", "doc"] {
        let dir_path = root.join(dir_name);
        if dir_path.is_dir() {
            if let Some(doc_tree) = build_doc_tree(&dir_path, &root) {
                docs.push(doc_tree);
            }
        }
    }

    Ok(docs)
}

fn build_doc_tree(path: &PathBuf, root: &PathBuf) -> Option<DocFile> {
    let name = path.file_name()?.to_str()?.to_string();
    let relative_path = path.strip_prefix(root).ok()?.to_str()?.replace('\\', "/");
    
    if path.is_dir() {
        let mut children = Vec::new();
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Some(child) = build_doc_tree(&entry.path(), root) {
                    children.push(child);
                }
            }
        }
        
        // Ne pas ajouter le dossier s'il est vide et n'est pas "docs" ou "doc"
        if children.is_empty() && name != "docs" && name != "doc" {
            return None;
        }

        children.sort_by(|a, b| {
            if a.is_dir != b.is_dir {
                b.is_dir.cmp(&a.is_dir) // Dossiers en premier
            } else {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
        });

        Some(DocFile {
            name,
            path: relative_path,
            is_dir: true,
            children: Some(children),
        })
    } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
        Some(DocFile {
            name,
            path: relative_path,
            is_dir: false,
            children: None,
        })
    } else {
        None
    }
}

#[tauri::command]
pub fn read_doc_file(path: String, workspace_id: Option<String>) -> Result<String, String> {
    let root = get_workspace_root(workspace_id)?;
    let full_path = root.join(&path);

    // Sécurité: s'assurer que le chemin ne sort pas de la racine et est bien un .md
    if !full_path.exists() {
        return Err(format!("Fichier introuvable: {}", path));
    }
    
    if full_path.extension().and_then(|s| s.to_str()) != Some("md") {
        return Err("Seuls les fichiers Markdown sont autorisés".to_string());
    }

    fs::read_to_string(full_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_doc_image(path: String, workspace_id: Option<String>) -> Result<Vec<u8>, String> {
    let root = get_workspace_root(workspace_id)?;
    let full_path = root.join(&path);

    // Sécurité: s'assurer que le chemin ne sort pas de la racine
    if !full_path.exists() {
        return Err(format!("Image introuvable: {}", path));
    }

    // Vérifier les extensions d'images communes
    let ext = full_path.extension().and_then(|s| s.to_str()).unwrap_or_default().to_lowercase();
    let allowed_exts = ["png", "jpg", "jpeg", "gif", "svg", "webp"];
    if !allowed_exts.contains(&ext.as_str()) {
        return Err("Type de fichier non autorisé".to_string());
    }

    fs::read(full_path).map_err(|e| e.to_string())
}
