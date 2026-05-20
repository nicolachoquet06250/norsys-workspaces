import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { ask, message } from '@tauri-apps/plugin-dialog';

export async function checkForAppUpdates(manual = false) {
  try {
    const update = await check();
    
    if (update) {
      console.log(`Mise à jour disponible : ${update.version} du ${update.date}`);
      
      const yes = await ask(
        `Une nouvelle version (${update.version}) est disponible. Souhaitez-vous l'installer maintenant ?\n\nNotes de version :\n${update.body}`,
        {
          title: 'Mise à jour disponible',
          kind: 'info',
          okLabel: 'Mettre à jour',
          cancelLabel: 'Plus tard',
        }
      );

      if (yes) {
        console.log('Téléchargement et installation de la mise à jour...');
        await update.downloadAndInstall();
        
        await message('La mise à jour a été installée avec succès. L\'application va redémarrer.', {
          title: 'Mise à jour terminée',
          kind: 'info',
        });
        
        await relaunch();
      }
    } else if (manual) {
      await message('Votre application est déjà à jour.', {
        title: 'Pas de mise à jour',
        kind: 'info',
      });
    }
  } catch (error) {
    console.error('Erreur lors de la recherche de mise à jour :', error);
    if (manual) {
      await message(`Erreur lors de la recherche de mise à jour : ${error}`, {
        title: 'Erreur',
        kind: 'error',
      });
    }
  }
}
